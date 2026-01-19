use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, State},
    routing::{get, post},
};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

use crate::config;

pub struct AppState {
    pub bearer_tokens: Vec<String>,
    pub config: config::Config,
}

pub async fn start(bearer_tokens: Vec<String>, config: config::Config) {
    // TODO: create a route per configured project

    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { "Hello from `build-hook`!" }))
        .route("/{project}", post(handler))
        .with_state(Arc::new(AppState {
            bearer_tokens,
            config,
        }))
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

    tracing::info!("Server starting on 0.0.0.0:3000");

    // TODO: use the k8s buildx target for image builds

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
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
