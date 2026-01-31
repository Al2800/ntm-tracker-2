use clap::{Parser, Subcommand};
use ntm_tracker_daemon::bus::EventBus;
use ntm_tracker_daemon::cache::{Cache, PollingDatum};
use ntm_tracker_daemon::cli::{self, OutputFormat, DEFAULT_PORT};
use ntm_tracker_daemon::collector::ntm::{NtmCollector, NtmCollectorConfig};
use ntm_tracker_daemon::collector::tmux::{TmuxCollector, TmuxCollectorConfig};
use ntm_tracker_daemon::command::{CommandConfig, CommandRunner};
use ntm_tracker_daemon::config::{ConfigManager, PollingConfig};
use ntm_tracker_daemon::logging;
use ntm_tracker_daemon::maintenance;
use ntm_tracker_daemon::ntm::{NtmClient, NtmConfig};
use ntm_tracker_daemon::rpc::handlers;
use ntm_tracker_daemon::rpc::RpcContext;
use ntm_tracker_daemon::service::{InstanceGuard, ShutdownHandler};
use ntm_tracker_daemon::transport;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, Parser)]
#[command(name = "ntm-tracker-daemon", version, about = "NTM Tracker daemon")]
struct Args {
    /// Output in JSON format.
    #[arg(long, global = true)]
    json: bool,

    /// Increase verbosity.
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Optional config file override (TOML).
    #[arg(long, global = true)]
    config: Option<std::path::PathBuf>,

    /// Port to connect to for client commands.
    #[arg(long, global = true, default_value_t = DEFAULT_PORT)]
    port: u16,

    /// Admin token for privileged operations.
    #[arg(long, global = true)]
    admin_token: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Start the daemon (default if no command specified).
    Start {
        /// Log level (trace, debug, info, warn, error). Overrides config.
        #[arg(long)]
        log_level: Option<String>,

        /// Log format: "text" or "json". Overrides config.
        #[arg(long)]
        log_format: Option<String>,

        /// Use stdio transport (newline-delimited JSON-RPC over stdin/stdout).
        #[arg(long)]
        stdio: bool,

        /// Start WebSocket server on specified port.
        #[arg(long)]
        ws_port: Option<u16>,

        /// Start HTTP server on specified port.
        #[arg(long)]
        http_port: Option<u16>,

        /// Allow multiple daemon instances (for testing).
        #[arg(long)]
        no_single_instance: bool,
    },

    /// Stop the running daemon.
    Stop,

    /// Show daemon health status.
    Health,

    /// Show session summary.
    Status,

    /// List recent events.
    Events {
        /// Maximum number of events to show.
        #[arg(long, short = 'n', default_value_t = 20)]
        limit: u32,
    },

    /// Show or modify configuration.
    Config,

    /// Run self-test checks.
    SelfTest,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let format = if args.json {
        OutputFormat::Json
    } else {
        OutputFormat::Text
    };

    // Default to Start command if none specified
    let command = args.command.unwrap_or(Command::Start {
        log_level: None,
        log_format: None,
        stdio: false,
        ws_port: None,
        http_port: None,
        no_single_instance: false,
    });

    match command {
        Command::Start {
            log_level,
            log_format,
            stdio,
            ws_port,
            http_port,
            no_single_instance,
        } => {
            run_daemon(args.config, log_level, log_format, stdio, ws_port, http_port, no_single_instance).await;
        }

        Command::Stop => {
            if let Err(e) = cli::cmd_stop(None) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }

        Command::Health => {
            if let Err(e) = cli::cmd_health(args.port, format, args.admin_token) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }

        Command::Status => {
            if let Err(e) = cli::cmd_status(args.port, format, args.admin_token) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }

        Command::Events { limit } => {
            if let Err(e) = cli::cmd_events(args.port, format, args.admin_token, Some(limit)) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }

        Command::Config => {
            if let Err(e) = cli::cmd_config(args.port, format, args.admin_token) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }

        Command::SelfTest => {
            if let Err(e) = cli::cmd_self_test(args.port, format, args.admin_token) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
    }
}

async fn run_daemon(
    config_path: Option<std::path::PathBuf>,
    log_level: Option<String>,
    log_format: Option<String>,
    stdio: bool,
    ws_port: Option<u16>,
    http_port: Option<u16>,
    no_single_instance: bool,
) {
    // Acquire single-instance lock (unless disabled for testing)
    let _instance_guard = if no_single_instance {
        None
    } else {
        match InstanceGuard::acquire() {
            Ok(guard) => Some(guard),
            Err(err) => {
                eprintln!("Error: {err}");
                std::process::exit(1);
            }
        }
    };

    // Load config first (with basic stderr output for errors)
    let config = match ConfigManager::load_from_fs(config_path) {
        Ok(manager) => manager,
        Err(err) => {
            eprintln!("Error: Failed to load config: {err}");
            std::process::exit(2);
        }
    };

    // Build logging config from config file + CLI overrides
    let mut log_config = config.current().logging;
    if let Some(level) = log_level {
        log_config.level = level;
    }
    if let Some(format) = log_format {
        log_config.format = format;
    }

    // Initialize structured logging (keep guard alive for duration of program)
    let _log_guard = logging::init(&log_config);

    let admin_credential = match load_admin_credential(&config) {
        Ok(credential) => credential,
        Err(err) => {
            tracing::error!(error = %err, "Failed to load admin token");
            std::process::exit(2);
        }
    };

    tracing::info!(
        daemon = ntm_tracker_daemon::APP_NAME,
        version = ntm_tracker_daemon::version(),
        config_path = %config_path_str(&config),
        log_level = %log_config.level,
        log_format = %log_config.format,
        "daemon bootstrap"
    );

    // Create shared state
    let cache = Arc::new(Cache::new(1000));
    let ctx = Arc::new(RpcContext::new(cache, config));

    // Create shutdown handler for graceful shutdown
    let shutdown_handler = ShutdownHandler::new();

    let maintenance_runner = maintenance::MaintenanceRunner::new(
        ntm_tracker_daemon::service::data_dir().join("ntm-tracker.db"),
        ctx.config.current().maintenance,
    );
    let maintenance_shutdown = shutdown_handler.subscribe();
    tokio::spawn(async move {
        maintenance_runner.run_loop(maintenance_shutdown).await;
    });

    if ctx.capabilities.ntm {
        let ntm_shutdown = shutdown_handler.subscribe();
        spawn_ntm_collector(ctx.clone(), ntm_shutdown);
    } else {
        tracing::info!("NTM not detected; skipping NTM collector");
    }

    if ctx.capabilities.tmux {
        let tmux_shutdown = shutdown_handler.subscribe();
        spawn_tmux_collector(ctx.clone(), tmux_shutdown);
    } else {
        tracing::info!("tmux not detected; skipping tmux collector");
    }

    // Determine which transports to start
    let use_stdio = stdio || (ws_port.is_none() && http_port.is_none());

    // Spawn WS server if requested
    if let Some(port) = ws_port {
        let ws_config = transport::ws::WsConfig {
            port,
            admin_credential: admin_credential.clone(),
            tokens: Vec::new(),
        };
        let ws_server = transport::ws::WsServer::new(ws_config);
        let ws_ctx = ctx.clone();
        tokio::spawn(async move {
            let _ = ws_server.run(ws_ctx).await;
        });
    }

    // Spawn HTTP server if requested
    if let Some(port) = http_port {
        let http_config = transport::http::HttpConfig {
            port,
            admin_credential: admin_credential.clone(),
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
        let snapshot_shutdown = shutdown_handler.subscribe();
        spawn_stdio_snapshot_notifier(ctx.clone(), notif_tx.clone(), snapshot_shutdown);

        transport::stdio::run(ctx, notif_rx).await;
    } else {
        // If WS or HTTP is running, we need to keep the main task alive
        // Wait for shutdown signal (SIGTERM, SIGINT)
        tracing::info!("Running with WS/HTTP transports, waiting for shutdown signal");
        shutdown_handler.wait_for_signal().await;

        // Allow graceful shutdown (1 second timeout)
        shutdown_handler
            .graceful_shutdown(std::time::Duration::from_secs(1))
            .await;
    }

    tracing::info!("Daemon shutdown complete");
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PollingMode {
    Active,
    Idle,
    Background,
    Degraded,
}

impl PollingMode {
    fn as_str(&self) -> &'static str {
        match self {
            PollingMode::Active => "active",
            PollingMode::Idle => "idle",
            PollingMode::Background => "background",
            PollingMode::Degraded => "degraded",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct PollingDecision {
    mode: PollingMode,
    reason: &'static str,
    interval_ms: u64,
}

fn compute_polling_decision(
    cache: &Cache,
    polling: &PollingConfig,
    error_streak: u32,
) -> PollingDecision {
    let now = current_unix_ts();
    let sessions = cache.all_sessions();
    let has_sessions = !sessions.is_empty();
    let is_active = sessions.iter().any(|session| {
        session.ended_at.is_none()
            && now.saturating_sub(session.last_seen_at) <= polling.idle_threshold_secs
    });

    let mut mode = if !has_sessions {
        PollingMode::Background
    } else if is_active {
        PollingMode::Active
    } else {
        PollingMode::Idle
    };

    let mut reason = match mode {
        PollingMode::Active => "recent_activity",
        PollingMode::Idle => "idle_timeout",
        PollingMode::Background => "no_sessions",
        PollingMode::Degraded => "degraded",
    };

    let mut interval_ms = match mode {
        PollingMode::Active => polling.snapshot_interval_ms,
        PollingMode::Idle => polling.snapshot_idle_interval_ms,
        PollingMode::Background => polling.snapshot_background_interval_ms,
        PollingMode::Degraded => polling.snapshot_degraded_interval_ms,
    };

    let health = cache.health();
    if error_streak > 0 {
        mode = PollingMode::Degraded;
        reason = "poll_errors";
        interval_ms = polling.snapshot_degraded_interval_ms;
    } else if !health.status.trim().is_empty() && health.status != "ok" {
        mode = PollingMode::Degraded;
        reason = "health_degraded";
        interval_ms = polling.snapshot_degraded_interval_ms;
    }

    interval_ms = interval_ms.max(250);

    PollingDecision {
        mode,
        reason,
        interval_ms,
    }
}

fn spawn_ntm_collector(
    ctx: Arc<RpcContext>,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) {
    tokio::spawn(async move {
        let polling = ctx.config.current().polling;
        let collector_config = NtmCollectorConfig {
            active_interval: std::time::Duration::from_millis(polling.snapshot_interval_ms),
            idle_interval: std::time::Duration::from_millis(polling.snapshot_idle_interval_ms),
            idle_threshold_secs: polling.idle_threshold_secs,
        };
        let runner = CommandRunner::new(CommandConfig::default());
        let client = NtmClient::new(runner, NtmConfig::default());
        let bus = EventBus::new(8);
        let mut collector = NtmCollector::new(client, bus, ctx.cache.clone(), collector_config);

        let mut error_streak = 0u32;
        loop {
            let polling = ctx.config.current().polling;
            let decision = compute_polling_decision(ctx.cache.as_ref(), &polling, error_streak);
            let now = current_unix_ts();
            let updated = ctx.cache.update_polling_ntm(PollingDatum {
                interval_ms: decision.interval_ms,
                mode: decision.mode.as_str().to_string(),
                reason: decision.reason.to_string(),
                last_change_at: now,
            });
            if updated {
                tracing::info!(
                    kind = "ntm",
                    interval_ms = decision.interval_ms,
                    mode = %decision.mode.as_str(),
                    reason = decision.reason,
                    "polling interval updated"
                );
            }

            let sleep = tokio::time::sleep(std::time::Duration::from_millis(decision.interval_ms));
            tokio::pin!(sleep);
            tokio::select! {
                _ = &mut sleep => {
                    match collector.poll_once().await {
                        Ok(result) => {
                            if result.degraded {
                                error_streak = error_streak.saturating_add(1);
                            } else {
                                error_streak = 0;
                            }
                        }
                        Err(err) => {
                            error_streak = error_streak.saturating_add(1);
                            tracing::warn!(error = %err, "ntm poll failed");
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    break;
                }
            }
        }
    });
}

fn spawn_tmux_collector(
    ctx: Arc<RpcContext>,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) {
    tokio::spawn(async move {
        let polling = ctx.config.current().polling;
        let collector_config = TmuxCollectorConfig {
            poll_interval: std::time::Duration::from_millis(polling.snapshot_interval_ms),
            ..TmuxCollectorConfig::default()
        };
        let runner = CommandRunner::new(CommandConfig::default());
        let bus = EventBus::new(8);
        let mut collector = TmuxCollector::new(runner, bus, ctx.cache.clone(), collector_config);

        let mut error_streak = 0u32;
        loop {
            let polling = ctx.config.current().polling;
            let decision = compute_polling_decision(ctx.cache.as_ref(), &polling, error_streak);
            let now = current_unix_ts();
            let updated = ctx.cache.update_polling_tmux(PollingDatum {
                interval_ms: decision.interval_ms,
                mode: decision.mode.as_str().to_string(),
                reason: decision.reason.to_string(),
                last_change_at: now,
            });
            if updated {
                tracing::info!(
                    kind = "tmux",
                    interval_ms = decision.interval_ms,
                    mode = %decision.mode.as_str(),
                    reason = decision.reason,
                    "polling interval updated"
                );
            }

            let sleep = tokio::time::sleep(std::time::Duration::from_millis(decision.interval_ms));
            tokio::pin!(sleep);
            tokio::select! {
                _ = &mut sleep => {
                    match collector.poll_once().await {
                        Ok(result) => {
                            if result.degraded {
                                error_streak = error_streak.saturating_add(1);
                            } else {
                                error_streak = 0;
                            }
                        }
                        Err(err) => {
                            error_streak = error_streak.saturating_add(1);
                            tracing::warn!(error = %err, "tmux poll failed");
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    break;
                }
            }
        }
    });
}

fn spawn_stdio_snapshot_notifier(
    ctx: Arc<RpcContext>,
    notification_tx: mpsc::Sender<transport::JsonRpcNotification>,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) {
    tokio::spawn(async move {
        let mut error_streak = 0u32;
        loop {
            let polling = ctx.config.current().polling;
            let decision = compute_polling_decision(ctx.cache.as_ref(), &polling, error_streak);
            let now = current_unix_ts();
            let updated = ctx.cache.update_polling_snapshot(PollingDatum {
                interval_ms: decision.interval_ms,
                mode: decision.mode.as_str().to_string(),
                reason: decision.reason.to_string(),
                last_change_at: now,
            });
            if updated {
                tracing::info!(
                    kind = "snapshot",
                    interval_ms = decision.interval_ms,
                    mode = %decision.mode.as_str(),
                    reason = decision.reason,
                    "polling interval updated"
                );
            }

            let sleep = tokio::time::sleep(std::time::Duration::from_millis(decision.interval_ms));
            tokio::pin!(sleep);
            tokio::select! {
                _ = &mut sleep => {
                    match handlers::core::snapshot_get(ctx.as_ref()) {
                        Ok(snapshot) => {
                            error_streak = 0;
                            let notification = transport::JsonRpcNotification::new("sessions.snapshot", snapshot);
                            if notification_tx.send(notification).await.is_err() {
                                break;
                            }
                        }
                        Err(err) => {
                            error_streak = error_streak.saturating_add(1);
                            tracing::warn!(error = %err.message, "snapshot notification failed");
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    break;
                }
            }
        }
    });
}

fn current_unix_ts() -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0));
    now.as_secs() as i64
}

fn config_path_str(config: &ConfigManager) -> String {
    config
        .config_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<defaults>".to_string())
}

fn load_admin_credential(config: &ConfigManager) -> Result<Option<String>, String> {
    let credential_path = config.current().security.admin_token_path;
    let Some(path) = credential_path else {
        return Ok(None);
    };

    let raw = std::fs::read_to_string(&path)
        .map_err(|err| format!("Unable to read admin token file '{}': {err}", path.display()))?;
    let credential = raw.trim().to_string();
    if credential.is_empty() {
        return Err(format!(
            "Admin token file '{}' is empty",
            path.display()
        ));
    }
    Ok(Some(credential))
}
