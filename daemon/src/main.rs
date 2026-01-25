use clap::{Parser, Subcommand};
use ntm_tracker_daemon::cache::Cache;
use ntm_tracker_daemon::cli::{self, OutputFormat, DEFAULT_PORT};
use ntm_tracker_daemon::config::ConfigManager;
use ntm_tracker_daemon::logging;
use ntm_tracker_daemon::rpc::RpcContext;
use ntm_tracker_daemon::service::{InstanceGuard, ShutdownHandler};
use ntm_tracker_daemon::transport;
use std::sync::Arc;

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

    // Determine which transports to start
    let use_stdio = stdio || (ws_port.is_none() && http_port.is_none());

    // Spawn WS server if requested
    if let Some(port) = ws_port {
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
    if let Some(port) = http_port {
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

fn config_path_str(config: &ConfigManager) -> String {
    config
        .config_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<defaults>".to_string())
}
