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
use tokio::sync::Semaphore;
use tower_http::trace::TraceLayer;

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

/// Assemble the router.
///
/// Split out from `start` so the routing and auth wiring can be tested without
/// binding a socket or reaching buildkitd -- `buildx::initialize()` blocks when
/// the builder endpoint is unreachable, so running the real binary is not a
/// viable way to test which routes require authentication.
pub fn build_router(
    config: config::HookConfig,
    github_token: String,
    bearer_tokens: Arc<auth::BearerTokens>,
) -> Router {
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

    // Protected routes (auth required). The middleware carries its own state --
    // the tokens loaded once at startup -- rather than re-reading the
    // environment on every request.
    let protected_routes = Router::new()
        .route("/{project}", post(handler))
        .route_layer(middleware::from_fn_with_state(
            bearer_tokens,
            auth::auth_layer,
        ));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(app_state)
        .layer(TraceLayer::new_for_http())
}

pub async fn start(
    config: config::HookConfig,
    github_token: String,
    bearer_tokens: Arc<auth::BearerTokens>,
) {
    let app = build_router(config, github_token, bearer_tokens);

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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    const CONFIG: &str = r#"
[app]
registry = "registry.example.test"

[[projects]]
name = "Test Project"
slug = "test"

[projects.code]
url = "https://github.com/example/test"
branch = "main"

[[projects.image]]
repository = "test/api"
location = "Dockerfile"
tag = "latest"

[projects.deployments]
namespace = "test"
resources = ["deployment/test"]
"#;

    fn router() -> Router {
        let config = config::load_from_str(CONFIG).expect("test config should load");
        let tokens = Arc::new(auth::BearerTokens::parse("goodtoken").expect("tokens should parse"));
        build_router(config, String::new(), tokens)
    }

    async fn status_of(req: Request<Body>) -> StatusCode {
        router()
            .oneshot(req)
            .await
            .expect("router responds")
            .status()
    }

    fn post(path: &str, auth_header: Option<&str>) -> Request<Body> {
        let mut b = Request::builder().method("POST").uri(path);
        if let Some(value) = auth_header {
            b = b.header("Authorization", value);
        }
        b.body(Body::empty()).unwrap()
    }

    #[tokio::test]
    async fn health_is_public() {
        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        assert_eq!(status_of(req).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn build_requires_authentication() {
        assert_eq!(
            status_of(post("/test", None)).await,
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn build_rejects_a_wrong_token() {
        assert_eq!(
            status_of(post("/test", Some("Bearer wrongtoken"))).await,
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn build_rejects_a_non_bearer_scheme() {
        assert_eq!(
            status_of(post("/test", Some("Basic goodtoken"))).await,
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn unknown_project_is_404_even_with_a_valid_token() {
        // The allowlist is the control that stops a valid token building an
        // arbitrary repository. This test is the one that must never regress.
        assert_eq!(
            status_of(post("/nosuchproject", Some("Bearer goodtoken"))).await,
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn unknown_project_is_unauthorized_without_a_token() {
        // Auth must be evaluated before the project lookup, so an unauthenticated
        // caller cannot probe which project slugs exist.
        assert_eq!(
            status_of(post("/nosuchproject", None)).await,
            StatusCode::UNAUTHORIZED
        );
    }
}
