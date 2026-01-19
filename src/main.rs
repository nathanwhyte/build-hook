use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, State},
    routing::{get, post},
};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;

struct AppState {
    bearer_tokens: Vec<String>,
    config: config::Config,
}

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

    let app_state = Arc::new(AppState {
        bearer_tokens,
        config,
    });

    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { "Hello from `build-hook`!" }))
        .route("/{project}", post(handler))
        .with_state(app_state)
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

    tracing::info!("Server starting on 0.0.0.0:3000");

    // TODO: use the k8s buildx target for image builds

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

async fn handler(Path(path): Path<String>, State(state): State<Arc<AppState>>) -> String {
    tracing::debug!("Using tokens: {:?}", state.bearer_tokens);

    match state.config.get(&path) {
        Some(project_config) => {
            tracing::info!(
                "Received build hook for project `{}`, building...",
                project_config.name
            );
            format!(
                "Received build hook for project `{}`, building...",
                project_config.name
            )
        }
        None => {
            tracing::warn!("No configuration found for project `{}`, skipping...", path);
            format!("No configuration found for project `{}`, skipping...", path)
        }
    }
}
