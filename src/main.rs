use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod config;

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

    // read in comma-separated list of bearer tokens from env
    tracing::debug!("Loading bearer tokens...");
    let bearer_tokens = load_env();

    // read in env and config
    tracing::debug!("Loading config...");
    let config = match config::load() {
        Ok(cfg) => cfg,
        Err(e) => panic!("Could not load config: {}", e),
    };

    api::start(bearer_tokens, config).await;
}

fn load_env() -> Vec<String> {
    // NOTE: temporary, bacon doesn't support env in `run` jobs
    let tokens_string = std::env::var("BEARER_TOKENS").unwrap_or("token12345".to_string());

    // let tokens_string = match std::env::var("BEARER_TOKENS") {
    //     Ok(tokens) => tokens,
    //     Err(_) => panic!("No BEARER_TOKENS environment variable set!"),
    // };

    tokens_string
        .split(',')
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>()
}
