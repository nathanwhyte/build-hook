use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod auth;
mod buildx;
mod config;
mod kube;
mod project;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "build_hook=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // read in env and config
    tracing::debug!("Loading config...");
    let config = match config::load() {
        Ok(cfg) => cfg,
        Err(e) => {
            tracing::error!("Could not load config: {}", e);
            return;
        }
    };

    let github_token = std::env::var("GITHUB_TOKEN").unwrap_or_default();
    if github_token.is_empty() {
        tracing::warn!(
            "No GITHUB_TOKEN environment variable set, git operations may fail if authentication is required."
        );
    }

    // Initialize buildx builder
    tracing::debug!("Initializing buildx...");
    if let Err(e) = buildx::initialize() {
        tracing::warn!(
            "Failed to initialize buildx builder: {}. Builds will fail until this is resolved.",
            e
        );
    }

    api::start(config, github_token).await;
}
