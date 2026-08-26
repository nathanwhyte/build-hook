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

    // read in env and config, exit if config is invalid in any way
    let config = match config::load() {
        Ok(cfg) => cfg,
        Err(e) => {
            // exit(1), not `return` -- returning from main exits 0, so a
            // container that cannot read its config reported SUCCESS to the
            // orchestrator and to anything keying on exit status.
            tracing::error!("Refusing to start, could not load config: {}", e);
            std::process::exit(1);
        }
    };

    // Load inbound auth BEFORE binding a listener. A server that starts without
    // usable bearer tokens looks healthy on /health and only fails when a build
    // is requested, which is how this whole pipeline stayed broken unnoticed
    // once already. Fail loudly at startup instead.
    let bearer_tokens = match auth::BearerTokens::from_env() {
        Ok(tokens) => {
            tracing::info!("Loaded {} inbound bearer token(s)", tokens.len());
            std::sync::Arc::new(tokens)
        }
        Err(e) => {
            tracing::error!("Refusing to start: {}", e);
            std::process::exit(1);
        }
    };

    let github_token = std::env::var("GITHUB_TOKEN").unwrap_or_default();
    if github_token.is_empty() {
        tracing::warn!(
            "No GITHUB_TOKEN environment variable set, git operations may fail if authentication is required."
        );
    }

    // Initialize buildx builder
    if let Err(e) = buildx::initialize() {
        tracing::warn!(
            "Failed to initialize buildx builder: {}. Builds will fail until this is resolved.",
            e
        );
    }

    api::start(config, github_token, bearer_tokens).await;
}
