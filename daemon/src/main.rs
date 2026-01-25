use clap::Parser;
use ntm_tracker_daemon::cache::Cache;
use ntm_tracker_daemon::config::ConfigManager;
use ntm_tracker_daemon::rpc::RpcContext;
use ntm_tracker_daemon::transport;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "ntm-tracker-daemon", version, about = "NTM Tracker daemon")]
struct Args {
    /// Optional config file override (TOML).
    #[arg(long)]
    config: Option<std::path::PathBuf>,

    /// Log level (trace, debug, info, warn, error).
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Use stdio transport (newline-delimited JSON-RPC over stdin/stdout).
    #[arg(long)]
    stdio: bool,

    /// Start WebSocket server on specified port.
    #[arg(long)]
    ws_port: Option<u16>,

    /// Start HTTP server on specified port.
    #[arg(long)]
    http_port: Option<u16>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    init_tracing(&args.log_level);

    let config = match ConfigManager::load_from_fs(args.config) {
        Ok(manager) => manager,
        Err(err) => {
            tracing::error!(error = %err, "failed to load config");
            std::process::exit(2);
        }
    };

    tracing::info!(
        daemon = ntm_tracker_daemon::APP_NAME,
        version = ntm_tracker_daemon::version(),
        config_path = %config_path(&config),
        "daemon bootstrap"
    );

    // Create shared state
    let cache = Arc::new(Cache::new(1000));
    let ctx = Arc::new(RpcContext::new(cache, config));

    // Determine which transports to start
    let use_stdio = args.stdio || (args.ws_port.is_none() && args.http_port.is_none());

    // Spawn WS server if requested
    if let Some(port) = args.ws_port {
        let ws_config = transport::ws::WsConfig {
            port,
            admin_token: None, // TODO: get from config
            tokens: Vec::new(),
        };
        let ws_server = transport::ws::WsServer::new(ws_config);
        let ws_ctx = ctx.clone();
        tokio::spawn(async move {
            let _ = ws_server.run(ws_ctx).await;
        });
    }

    // Spawn HTTP server if requested
    if let Some(port) = args.http_port {
        let http_config = transport::http::HttpConfig {
            port,
            admin_token: None, // TODO: get from config
            tokens: Vec::new(),
        };
        let http_server = transport::http::HttpServer::new(http_config);
        let http_ctx = ctx.clone();
        tokio::spawn(async move {
            let _ = http_server.run(http_ctx).await;
        });
    }

    if use_stdio {
        // stdio is the primary transport when no other is specified
        let (notif_tx, notif_rx) = transport::stdio::notification_channel();

        // Store the sender for later use by collectors/detectors
        // For now, we'll just drop it since we don't have event sources yet
        drop(notif_tx);

        transport::stdio::run(ctx, notif_rx).await;
    } else {
        // If WS or HTTP is running, we need to keep the main task alive
        // Use tokio::signal to wait for shutdown
        tracing::info!("Running with WS/HTTP transports, waiting for shutdown signal");
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                tracing::info!("Received shutdown signal");
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to wait for shutdown signal");
            }
        }
    }
}

fn config_path(config: &ConfigManager) -> String {
    config
        .config_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<defaults>".to_string())
}

fn init_tracing(level: &str) {
    let filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
