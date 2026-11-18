// tests/api/support.rs: what every API test shares
//
// Calls go straight into the router (no network). The database connection is the "disconnected"
// one, so a test that reaches the database panics: every test here must be answered before it.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use quoto::api::router;
use quoto::*;
use sea_orm::DatabaseConnection;
use serde_json::Value;
use tower::ServiceExt;

pub const KEY: &str = "test-key-test-key-test-key-test-key-1234";

pub fn app() -> Router {
    router(DatabaseConnection::default(), Secret::new(KEY.to_string()))
}

/// Makes one call and returns the status and the JSON that came back.
pub async fn call(
    method: &str,
    uri: &str,
    key: Option<&str>,
    body: Option<&str>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(key) = key {
        request = request.header(header::AUTHORIZATION, format!("Bearer {key}"));
    }
    if body.is_some() {
        request = request.header(header::CONTENT_TYPE, "application/json");
    }
    let request = request
        .body(Body::from(body.unwrap_or("").to_string()))
        .unwrap();
    let response = app().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1 << 20).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}
