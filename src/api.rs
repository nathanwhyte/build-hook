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
        (
            StatusCode::OK,
            "Build completed and rollout restart initiated\n",
        )
            .into_response()
    }
}

pub struct AppState {
    config: config::HookConfig,
    github_token: String,
}

pub async fn start(config: config::HookConfig, github_token: String) {
    let app_state = Arc::new(AppState {
        config,
        github_token,
    });

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

async fn handler(Path(slug): Path<String>, State(state): State<Arc<AppState>>) -> Response {
    match state.config.projects.get(&slug) {
        Some(project) => {
            tracing::info!(
                "Received build hook for project `{}`, building...",
                project.slug()
            );
            let registry = &state.config.app.registry;
            let github_token = &state.github_token;
            match project.build(registry, github_token) {
                Ok(()) => {
                    tracing::info!(
                        "Build completed and rollout restart initiated for project `{}`",
                        project.slug()
                    );
                    BuildHookResponse.into_response()
                }
                Err(e) => {
                    tracing::error!(
                        "Build or rollout restart failed for project `{}`: {}",
                        project.slug(),
                        e
                    );
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!(
                            "Build or rollout restart failed for project `{}`:\n{}\n",
                            project.slug(),
                            e
                        ),
                    )
                        .into_response()
                }
            }
        }

        None => {
            tracing::warn!("No configuration found for project `{}`, skipping...", slug);
            (
                StatusCode::NOT_FOUND,
                format!("No configuration found for project `{}`\n", slug),
            )
                .into_response()
        }
    }
}
