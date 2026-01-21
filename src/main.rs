use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod auth;
mod config;
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
        Err(e) => panic!("Could not load config: {}", e),
    };

    api::start(config).await;
}
