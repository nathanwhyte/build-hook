use axum::{Router, routing::post};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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
    let _bearer_tokens = load_env();

    // read in env and config
    tracing::debug!("Loading config...");
    let _config = match config::load() {
        Ok(cfg) => cfg,
        Err(e) => panic!("Could not load config: {}", e),
    };

    // build our application with a single route
    let app = Router::new()
        .route("/", post(|| async { "Hello, World!" }))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
        );

    tracing::info!("Server starting on 0.0.0.0:3000");

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
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
