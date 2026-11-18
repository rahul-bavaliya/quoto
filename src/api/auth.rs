// src/api/auth.rs: every call needs the API key (until real login arrives in Phase 9)

use super::AppState;
use super::error::ApiError;
use axum::extract::{Request, State};
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;
use subtle::ConstantTimeEq;

/// Lets the call through only if it carries `Authorization: Bearer <the key>`.
/// The two keys are compared in constant time, so the answer time doesn't hint how many
/// characters were right.
pub(super) async fn require_key(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let given = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    match given {
        Some(given) if bool::from(given.as_bytes().ct_eq(state.key.expose().as_bytes())) => {
            Ok(next.run(request).await)
        }
        _ => Err(ApiError::Unauthorized),
    }
}
