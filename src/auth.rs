use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use sha2::{Digest, Sha256};
use subtle::{Choice, ConstantTimeEq};
use tokio::task_local;

#[derive(Clone)]
pub struct CurrentUser;

task_local! {
    pub static USER: CurrentUser;
}

/// Inbound webhook bearer tokens.
///
/// Loaded ONCE at startup rather than per-request. The previous version read
/// `BEARER_TOKENS` inside the request path and panicked if it was unset, which
/// meant a deployment missing the secret started cleanly, served `/health` with
/// a 200, and only failed when someone actually asked for a build. Loading here
/// turns that misconfiguration into a startup failure you cannot miss.
/// Stores SHA-256 digests, not the tokens themselves.
///
/// Comparing digests rather than raw tokens is what makes the check genuinely
/// constant time: `ct_eq` on slices returns early when the lengths differ, so
/// comparing raw tokens of differing lengths would let an attacker probing
/// candidate lengths infer which token lengths are configured. Every digest is
/// exactly 32 bytes, so every comparison does the same work.
///
/// Hashing the candidate is not itself length-independent — a longer candidate
/// takes more compression blocks — but that only reveals the length of the
/// value the attacker already chose. It leaks nothing about the configured
/// tokens.
///
/// Keeping digests also means the plaintext tokens are not retained in the
/// process after startup.
pub struct BearerTokens(Vec<[u8; 32]>);

impl BearerTokens {
    pub fn from_env() -> Result<Self, String> {
        let raw = std::env::var("BEARER_TOKENS")
            .map_err(|_| "BEARER_TOKENS environment variable is not set".to_string())?;
        Self::parse(&raw)
    }

    /// Parse a comma-separated token list.
    ///
    /// Split out from `from_env` so it is testable without mutating the process
    /// environment -- `cargo test` runs tests in parallel threads sharing one
    /// env, so env-mutating tests race and fail nondeterministically.
    pub fn parse(raw: &str) -> Result<Self, String> {
        let digests: Vec<[u8; 32]> = raw
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(Self::digest)
            .collect();

        if digests.is_empty() {
            return Err("BEARER_TOKENS is set but contains no non-empty tokens".to_string());
        }

        Ok(Self(digests))
    }

    fn digest(token: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        hasher.finalize().into()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Constant-time membership test over fixed-length digests.
    ///
    /// `Vec::contains` compares with `String`'s `PartialEq`, which short-circuits
    /// on the first differing byte and so leaks how much of a guess was correct.
    /// Here every configured digest is compared against the candidate's, always
    /// over the same 32 bytes, and the results accumulate into a `Choice` rather
    /// than a `bool` — so neither an individual comparison nor the loop exits
    /// early on a match.
    fn contains(&self, candidate: &str) -> bool {
        let candidate = Self::digest(candidate);
        let mut hit = Choice::from(0u8);
        for digest in &self.0 {
            hit |= digest.ct_eq(&candidate);
        }
        bool::from(hit)
    }
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

pub async fn auth_layer(
    State(tokens): State<Arc<BearerTokens>>,
    req: Request,
    next: Next,
) -> Response {
    let header_value = match req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
    {
        Some(value) => value,
        None => {
            tracing::warn!("Rejected request: missing Authorization header");
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
            tracing::warn!("Rejected request: malformed Authorization header");
            return (
                StatusCode::UNAUTHORIZED,
                "Unauthorized: Invalid Authorization header format. Expected 'Bearer <token>'",
            )
                .into_response();
        }
    };

    if !tokens.contains(token) {
        // Never log the presented token, not even a prefix -- a near-miss
        // prefix in a log is a meaningful hint to anyone who can read logs.
        tracing::warn!("Rejected request: invalid bearer token");
        return (
            StatusCode::UNAUTHORIZED,
            "Unauthorized: Invalid or missing bearer token",
        )
            .into_response();
    }

    USER.scope(CurrentUser, next.run(req)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(raw: &str) -> BearerTokens {
        BearerTokens::parse(raw).expect("should parse")
    }

    #[test]
    fn accepts_a_configured_token() {
        let t = tokens("alpha,beta");
        assert!(t.contains("alpha"));
        assert!(t.contains("beta"));
    }

    #[test]
    fn rejects_unknown_and_partial_tokens() {
        let t = tokens("alpha,beta");
        assert!(!t.contains("gamma"));
        assert!(!t.contains("alph"));
        assert!(!t.contains("alphaa"));
        assert!(!t.contains(""));
    }

    #[test]
    fn trims_whitespace_and_drops_empties() {
        let t = tokens(" alpha , , beta ");
        assert_eq!(t.len(), 2);
        assert!(t.contains("alpha"));
        assert!(!t.contains(""));
    }

    #[test]
    fn tokens_of_differing_lengths_are_all_compared_over_32_bytes() {
        // Regression guard for the PR #2 review finding: comparing raw tokens
        // makes ct_eq return early on a length mismatch, so probing candidate
        // lengths would reveal which lengths are configured. Everything is
        // hashed to a fixed width first, so a short and a long configured token
        // are indistinguishable in the work done per comparison.
        let t = tokens("short,a-considerably-longer-configured-token");
        assert!(t.contains("short"));
        assert!(t.contains("a-considerably-longer-configured-token"));
        assert!(!t.contains("s"));
        assert!(!t.contains("a-considerably-longer-configured-toke"));
        assert!(!t.contains("a-considerably-longer-configured-tokenX"));
    }

    #[test]
    fn plaintext_tokens_are_not_retained() {
        // The struct holds digests; a raw token must not survive in it.
        let t = tokens("supersecrettoken");
        let raw = b"supersecrettoken";
        assert!(
            !t.0.iter().any(|d| d.starts_with(&raw[..4])),
            "digest storage should not contain the plaintext token"
        );
    }

    #[test]
    fn empty_configuration_is_an_error_not_an_open_door() {
        assert!(BearerTokens::parse(" , ").is_err());
        assert!(BearerTokens::parse("").is_err());
        assert!(BearerTokens::parse(",,,").is_err());
    }

    #[test]
    fn parse_bearer_requires_scheme_and_nonempty_token() {
        assert_eq!(parse_bearer("Bearer abc"), Some("abc"));
        assert_eq!(parse_bearer("bearer abc"), Some("abc"));
        assert_eq!(parse_bearer("Bearer "), None);
        assert_eq!(parse_bearer("Basic abc"), None);
        assert_eq!(parse_bearer("abc"), None);
    }
}
