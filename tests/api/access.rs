// tests/api/access.rs: who may call what: the key, and the calls that do not exist

use super::support::{KEY, app, call};
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use tower::ServiceExt;

#[tokio::test]
async fn a_call_without_a_key_is_refused() {
    let (status, body) = call("GET", "/branches", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "UNAUTHORIZED");
}

#[tokio::test]
async fn a_wrong_key_is_refused_even_when_it_is_almost_right() {
    let almost = format!("{}5", &KEY[..KEY.len() - 1]);
    for wrong in ["wrong", "", &almost, &KEY[..KEY.len() - 1]] {
        let (status, _) = call("GET", "/branches", Some(wrong), None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{wrong:?}");
    }
}

#[tokio::test]
async fn the_key_must_come_as_a_bearer_token() {
    for header_value in [
        KEY.to_string(),
        format!("Basic {KEY}"),
        format!("bearer {KEY}"),
    ] {
        let request = Request::builder()
            .uri("/branches")
            .header(header::AUTHORIZATION, header_value)
            .body(Body::empty())
            .unwrap();
        let response = app().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
async fn a_refusal_says_how_to_log_in_and_never_repeats_the_key() {
    let request = Request::builder()
        .uri("/branches")
        .header(
            header::AUTHORIZATION,
            "Bearer secret-guess-secret-guess-123456",
        )
        .body(Body::empty())
        .unwrap();
    let response = app().oneshot(request).await.unwrap();
    assert_eq!(response.headers()[header::WWW_AUTHENTICATE], "Bearer");
    let bytes = to_bytes(response.into_body(), 1 << 20).await.unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(!text.contains("secret-guess") && !text.contains(KEY));
}

#[tokio::test]
async fn routes_that_do_not_exist_also_need_the_key() {
    // So a stranger can't learn which routes exist.
    let (status, _) = call("GET", "/nothing-here", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn branches_are_never_deleted() {
    let id = "00000000-0000-0000-0000-000000000000";
    for uri in [
        "/branches",
        &format!("/branches/{id}"),
        &format!("/branches/{id}/status"),
    ] {
        let (status, _) = call("DELETE", uri, Some(KEY), None).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{uri}");
    }
}
