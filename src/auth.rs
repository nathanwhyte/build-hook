use axum::{
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use tokio::task_local;

#[derive(Clone)]
pub struct CurrentUser;

task_local! {
    pub static USER: CurrentUser;
}

fn parse_bearer(header_value: &str) -> Option<&str> {
    // Authorization: Bearer <token>
    let (scheme, token) = header_value.split_once(' ')?;
    if scheme.eq_ignore_ascii_case("Bearer") && !token.is_empty() {
        Some(token)
    } else {
        None
    }
}

async fn authorize_bearer(token: &str) -> Option<CurrentUser> {
    let bearer_tokens = load_bearer_tokens_from_env();

    if !bearer_tokens.contains(&token.to_string()) {
        tracing::warn!("Invalid bearer token");
        return None;
    }

    tracing::info!("Valid bearer token");
    Some(CurrentUser)
}

pub async fn auth_layer(req: Request, next: Next) -> Response {
    tracing::info!("Authenticating request...");

    let header_value = match req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
    {
        Some(value) => value,
        None => {
            tracing::warn!("Missing Authorization header");
            return (
                StatusCode::UNAUTHORIZED,
                "Unauthorized: Missing Authorization header",
            )
                .into_response();
        }
    };

    let token = match parse_bearer(header_value) {
        Some(token) => token,
        None => {
            tracing::warn!("Invalid Authorization header format");
            return (
                StatusCode::UNAUTHORIZED,
                "Unauthorized: Invalid Authorization header format. Expected 'Bearer <token>'",
            )
                .into_response();
        }
    };

    let user = match authorize_bearer(token).await {
        Some(user) => user,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                "Unauthorized: Invalid or missing bearer token",
            )
                .into_response();
        }
    };

    tracing::debug!("Authorized user");

    USER.scope(user, next.run(req)).await
}

pub fn load_bearer_tokens_from_env() -> Vec<String> {
    let tokens_string = match std::env::var("BEARER_TOKENS") {
        Ok(tokens) => tokens,
        Err(_) => panic!("No BEARER_TOKENS environment variable set!"),
    };

    tokens_string
        .split(',')
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>()
}
