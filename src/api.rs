use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
};
use tower_http::trace::TraceLayer;

use crate::auth;
use crate::config;

pub struct BuildHookResponse;

impl IntoResponse for BuildHookResponse {
    fn into_response(self) -> Response {
        // State is accessed here in the IntoResponse implementation
        let current_user = auth::USER.with(|u| u.clone());
        (
            StatusCode::OK,
            format!("Hi there, user `{}`", current_user.id),
        )
            .into_response()
    }
}

pub struct AppState {
    pub config: config::Config,
}

pub async fn start(config: config::Config) {
    // TODO: create a route per configured project

    let app_state = Arc::new(AppState { config });

    // Public routes (no auth required)
    let public_routes = Router::new().route("/health", get(healthcheck));

    // Protected routes (auth required)
    let protected_routes = Router::new()
        .route("/{project}", post(handler))
        .route_layer(middleware::from_fn(auth::auth_layer));

    // build our application with public and protected routes
    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(app_state)
        .layer(TraceLayer::new_for_http());

    tracing::info!("Server starting on 0.0.0.0:3000");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn healthcheck() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok"
    }))
}

async fn handler(
    Path(slug): Path<String>,
    State(state): State<Arc<AppState>>,
) -> BuildHookResponse {
    match state.config.get(&slug) {
        Some(project_config) => {
            tracing::info!(
                "Received build hook for project `{}`, building...",
                project_config.slug()
            );
            BuildHookResponse
        }
        None => {
            tracing::warn!("No configuration found for project `{}`, skipping...", slug);
            BuildHookResponse
        }
    }
}
