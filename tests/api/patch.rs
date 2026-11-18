// tests/api/patch.rs: PATCH is checked before the database is touched

use super::support::{KEY, call};
use axum::http::StatusCode;

#[tokio::test]
async fn a_patch_with_nothing_to_change_is_refused_before_the_database() {
    let nil = "00000000-0000-0000-0000-000000000000";
    for uri in [format!("/branches/{nil}"), format!("/customers/{nil}")] {
        // An empty object, no body, not JSON, and JSON that is not an object: nothing to change.
        for body in ["{}", "", "not json", "[]", "\"phone\"", "null"] {
            let (status, answer) = call("PATCH", &uri, Some(KEY), Some(body)).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{uri} {body}");
            assert_eq!(answer["code"], "INVALID_REQUEST_BODY", "{uri} {body}");
        }
    }
}

#[tokio::test]
async fn a_patch_for_an_id_that_is_not_a_uuid_is_a_record_nobody_has() {
    let body = r#"{"phone":"306-555-0100"}"#;
    let (status, answer) = call("PATCH", "/branches/nope", Some(KEY), Some(body)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(answer["code"], "UNKNOWN_BRANCH");
    let (status, answer) = call("PATCH", "/customers/nope", Some(KEY), Some(body)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(answer["code"], "UNKNOWN_CUSTOMER");
}
