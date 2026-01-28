use std::collections::HashMap;
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
use tokio::sync::Semaphore;

use crate::auth;
use crate::config;

pub struct BuildHookResponse;

impl IntoResponse for BuildHookResponse {
    fn into_response(self) -> Response {
        // State is accessed here in the IntoResponse implementation
        (
            StatusCode::OK,
            "Build started; rollout restart will run after build completes\n",
        )
            .into_response()
    }
}

pub struct AppState {
    config: config::HookConfig,
    github_token: String,
    build_locks: HashMap<String, Arc<Semaphore>>,
}

pub async fn start(config: config::HookConfig, github_token: String) {
    let build_locks: HashMap<String, Arc<Semaphore>> = config
        .projects
        .keys()
        .map(|slug| (slug.clone(), Arc::new(Semaphore::new(1))))
        .collect();
    let app_state = Arc::new(AppState {
        config,
        github_token,
        build_locks,
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

    let listener = tokio::net::TcpListener::bind("0.0.0.0:5000").await.unwrap();
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
            let build_lock = match state.build_locks.get(&slug) {
                Some(lock) => Arc::clone(lock),
                None => {
                    tracing::error!("No build lock configured for project `{}`", slug);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Build lock missing for project `{}`\n", slug),
                    )
                        .into_response();
                }
            };
            let permit = match build_lock.try_acquire_owned() {
                Ok(permit) => permit,
                Err(_) => {
                    tracing::warn!("Build already in progress for project `{}`", slug);
                    return (
                        StatusCode::CONFLICT,
                        format!("Build already in progress for project `{}`\n", slug),
                    )
                        .into_response();
                }
            };

            let registry = &state.config.app.registry;
            let github_token = &state.github_token;
            let project = project.clone();
            let registry = registry.clone();
            let github_token = github_token.clone();
            let slug = project.slug().to_string();
            let slug_for_log = slug.clone();

            tokio::task::spawn_blocking(move || {
                let _permit = permit;
                if let Err(e) = project.build(&registry, &github_token) {
                    tracing::error!("Build failed for project `{}`: {}", slug, e);
                }
            });

            tracing::info!("Build started for project `{}`", slug_for_log);
            BuildHookResponse.into_response()
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
