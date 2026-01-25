use clap::Parser;
use ntm_tracker_daemon::config::ConfigManager;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "ntm-tracker-daemon", version, about = "NTM Tracker daemon")]
struct Args {
    /// Optional config file override (TOML).
    #[arg(long)]
    config: Option<std::path::PathBuf>,

    #[arg(long, default_value = "info")]
    log_level: String,
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
