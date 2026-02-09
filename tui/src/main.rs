use clap::Parser;
use ftui::{App, ScreenMode};
use ntm_tracker_tui::app::NtmApp;
use ntm_tracker_tui::msg::{self, Msg};
use ntm_tracker_tui::rpc::client::RpcClient;
use tracing::info;

/// NTM Tracker TUI — terminal dashboard for the NTM Tracker daemon.
#[derive(Parser)]
#[command(name = "ntm-tui", version = "0.1.0")]
struct Cli {
    /// Path to the daemon binary.
    #[arg(long, default_value = "ntm-tracker-daemon")]
    daemon_bin: String,

    /// Skip spawning the daemon (connect to existing instance).
    #[arg(long)]
    no_daemon: bool,

    /// Log file path.
    #[arg(long)]
    log_file: Option<String>,
}

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    // Set up file-based logging (stdout is used by the TUI).
    if let Some(log_path) = &cli.log_file {
        let path = std::path::Path::new(log_path);
        let dir = path.parent().unwrap_or(std::path::Path::new("."));
        let filename = path.file_name().unwrap_or_default();
        let file_appender = tracing_appender::rolling::never(dir, filename);
        tracing_subscriber::fmt()
            .with_writer(file_appender)
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "ntm_tracker_tui=debug".parse().unwrap()),
            )
            .init();
    }

    // Create the message channel (daemon → TUI).
    let (msg_tx, msg_rx) = tokio::sync::mpsc::unbounded_channel::<Msg>();
    let mut app = NtmApp::with_daemon_rx(msg_rx);

    // If not --no-daemon, spawn daemon and wire up RPC.
    if !cli.no_daemon {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        let _guard = rt.enter();

        match RpcClient::spawn(&cli.daemon_bin, msg_tx.clone()) {
            Ok(client) => {
                info!("Daemon spawned successfully");

                // Store write channel on app for fire-and-forget RPCs.
                app.set_rpc_tx(client.write_sender());

                // Request initial snapshot after short delay.
                let msg_tx2 = msg_tx.clone();
                rt.spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    match client.get_snapshot().await {
                        Ok(rx) => {
                            if let Ok(Ok(value)) = rx.await {
                                if let Ok(snap) = serde_json::from_value(value) {
                                    let _ = msg_tx2.send(Msg::SnapshotReceived(snap));
                                }
                            }
                        }
                        Err(e) => {
                            let _ = msg_tx2.send(Msg::RpcError(e));
                        }
                    }
                });

                // Keep the runtime alive.
                std::mem::forget(rt);
            }
            Err(e) => {
                app.conn_state = msg::ConnState::Error(format!("Spawn failed: {e}"));
            }
        }
    }

    // Launch the TUI.
    App::new(app)
        .screen_mode(ScreenMode::AltScreen)
        .run()
}
