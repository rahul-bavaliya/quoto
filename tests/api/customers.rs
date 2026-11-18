// tests/api/customers.rs: wrong customer input is refused before the database is touched

use super::support::{KEY, call};
use axum::http::StatusCode;
use quoto::*;
use serde_json::{Value, json};

fn customer_json() -> Value {
    json!({
        "name": "Jane Smith", "company": "Prairie Farms Ltd.", "designation": "Service Manager",
        "phone": "306-555-0142", "alternate_phone": "306-555-0143", "email": "jane@example.com",
        "address_line1": "123 Main St", "city": "Regina", "province": "SK", "postal_code": "S4P 3Y2"
    })
}

async fn post_customer(body: Value) -> (StatusCode, Value) {
    call("POST", "/customers", Some(KEY), Some(&body.to_string())).await
}

#[tokio::test]
async fn a_customer_body_that_is_not_the_expected_json_is_refused() {
    for body in ["", "not json", "[]", "{}", r#"{"name":"Jane"}"#] {
        let (status, answer) = call("POST", "/customers", Some(KEY), Some(body)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
        assert_eq!(answer["code"], "INVALID_REQUEST_BODY");
    }
}

#[tokio::test]
async fn a_caller_cannot_choose_the_status_or_the_id_of_a_new_customer() {
    for extra in ["status", "id"] {
        let mut body = customer_json();
        body[extra] = json!("inactive");
        let (status, answer) = post_customer(body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{extra}");
        assert_eq!(answer["code"], "INVALID_REQUEST_BODY");
    }
}

#[tokio::test]
async fn the_librarys_rules_answer_for_customers_with_their_own_codes() {
    let cases = [
        (
            "phone",
            json!("12345"),
            PhoneNumber::parse("12345").unwrap_err().code(),
        ),
        (
            "alternate_phone",
            json!("abc"),
            PhoneNumber::parse("abc").unwrap_err().code(),
        ),
        (
            "email",
            json!("not-an-email"),
            Email::parse("not-an-email").unwrap_err().code(),
        ),
        (
            "postal_code",
            json!("nope"),
            PostalCode::parse("nope").unwrap_err().code(),
        ),
        (
            "province",
            json!("ON"),
            Province::from_code("ON").unwrap_err().code(),
        ),
        (
            "name",
            json!("   "),
            QuoteError::MissingField {
                field: "customer name",
            }
            .code(),
        ),
        (
            "city",
            json!(""),
            QuoteError::MissingField { field: "city" }.code(),
        ),
    ];
    for (field, value, code) in cases {
        let mut body = customer_json();
        body[field] = value;
        let (status, answer) = post_customer(body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{field}");
        assert_eq!(answer["code"], code, "{field}");
    }
}

#[tokio::test]
async fn a_customer_status_that_is_not_active_or_inactive_is_refused() {
    let id = "00000000-0000-0000-0000-000000000000";
    let (status, answer) = call(
        "PATCH",
        &format!("/customers/{id}/status"),
        Some(KEY),
        Some(r#"{"status":"deleted"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(answer["code"], "INVALID_STATUS");

    let (status, _) = call("GET", "/customers?status=in_active", Some(KEY), None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_customer_id_that_is_not_a_uuid_is_a_customer_nobody_has() {
    let (status, answer) = call("GET", "/customers/not-an-id", Some(KEY), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(answer["code"], "UNKNOWN_CUSTOMER");
}

#[tokio::test]
async fn a_query_string_that_is_not_understood_is_one_plain_answer() {
    for uri in [
        "/customers?limit=many",
        "/customers?offset=-1",
        "/branches?status=active&status=inactive",
    ] {
        let (status, answer) = call("GET", uri, Some(KEY), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}");
        assert_eq!(answer["code"], "INVALID_QUERY", "{uri}");
    }
}
