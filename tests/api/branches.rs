// tests/api/branches.rs: wrong branch input is refused before the database is touched

use super::support::{KEY, call};
use axum::http::StatusCode;
use quoto::*;
use serde_json::{Value, json};

fn branch_json() -> Value {
    json!({
        "number": "T-01", "name": "Test Branch",
        "address_line1": "1 Industrial Rd", "city": "Regina", "province_code": "SK",
        "postal_code": "S4P 3Y2", "phone": "306-555-0100"
    })
}

async fn post(body: Value) -> (StatusCode, Value) {
    call("POST", "/branches", Some(KEY), Some(&body.to_string())).await
}

#[tokio::test]
async fn a_body_that_is_not_the_expected_json_is_refused() {
    for body in ["", "not json", "[]", "{}", r#"{"number":"T-01"}"#] {
        let (status, answer) = call("POST", "/branches", Some(KEY), Some(body)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
        assert_eq!(answer["code"], "INVALID_REQUEST_BODY");
    }
}

#[tokio::test]
async fn a_caller_cannot_choose_the_status_or_the_id_of_a_new_branch() {
    for extra in ["status", "id"] {
        let mut body = branch_json();
        body[extra] = json!("inactive");
        let (status, answer) = post(body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{extra}");
        assert_eq!(answer["code"], "INVALID_REQUEST_BODY");
    }
}

#[tokio::test]
async fn a_body_that_is_too_big_is_refused() {
    let mut body = branch_json();
    body["name"] = json!("x".repeat(20_000));
    let (status, _) = post(body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn the_librarys_rules_answer_with_their_own_codes() {
    let cases = [
        (
            "phone",
            json!("12345"),
            PhoneNumber::parse("12345").unwrap_err().code(),
        ),
        (
            "postal_code",
            json!("nope"),
            PostalCode::parse("nope").unwrap_err().code(),
        ),
        (
            "province_code",
            json!("ZZ"),
            Province::from_code("ZZ").unwrap_err().code(),
        ),
        (
            "name",
            json!("   "),
            AppError::MissingField {
                field: "branch name",
            }
            .code(),
        ),
        (
            "number",
            json!(""),
            AppError::MissingField {
                field: "branch number",
            }
            .code(),
        ),
    ];
    for (field, value, code) in cases {
        let mut body = branch_json();
        body[field] = value;
        let (status, answer) = post(body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{field}");
        assert_eq!(answer["code"], code, "{field}");
    }
}

#[tokio::test]
async fn half_a_map_location_is_refused() {
    let mut body = branch_json();
    body["latitude"] = json!(50.4);
    let (status, answer) = post(body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(answer["code"], AppError::InvalidLocation.code());
}

#[tokio::test]
async fn a_status_that_is_not_active_or_inactive_is_refused() {
    let id = "00000000-0000-0000-0000-000000000000";
    let (status, answer) = call(
        "PATCH",
        &format!("/branches/{id}/status"),
        Some(KEY),
        Some(r#"{"status":"deleted"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(answer["code"], "INVALID_STATUS");

    let (status, _) = call("GET", "/branches?status=in_active", Some(KEY), None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn an_id_that_is_not_a_uuid_is_a_branch_nobody_has() {
    let (status, answer) = call("GET", "/branches/not-an-id", Some(KEY), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(answer["code"], "UNKNOWN_BRANCH");
}
