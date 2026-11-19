// tests/api/repair_orders.rs: the repair order calls that are answered before the database is touched

use super::support::{KEY, call, call_with_header};
use axum::http::StatusCode;

const NIL: &str = "00000000-0000-0000-0000-000000000000";

async fn refused(method: &str, uri: &str, body: Option<&str>) -> (StatusCode, String) {
    let (status, answer) = call(method, uri, Some(KEY), body).await;
    (status, answer["code"].as_str().unwrap_or("?").to_string())
}

#[tokio::test]
async fn every_repair_order_call_needs_the_key() {
    for (method, uri) in [
        ("POST", format!("/quotes/{NIL}/repair-order")),
        ("GET", "/repair-orders".to_string()),
        ("GET", format!("/repair-orders/{NIL}")),
        ("PATCH", format!("/repair-orders/{NIL}")),
        ("POST", format!("/repair-orders/{NIL}/start")),
        ("POST", format!("/repair-orders/{NIL}/complete")),
        ("POST", format!("/repair-orders/{NIL}/cancel")),
    ] {
        let (status, _) = call(method, &uri, None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}");
    }
}

#[tokio::test]
async fn a_repair_order_cannot_be_deleted_replaced_created_directly_or_given_a_status() {
    for (method, uri) in [
        ("DELETE", format!("/repair-orders/{NIL}")),
        ("PUT", format!("/repair-orders/{NIL}")),
        ("POST", "/repair-orders".to_string()),
        ("DELETE", "/repair-orders".to_string()),
        ("PATCH", format!("/repair-orders/{NIL}/status")),
        ("GET", format!("/repair-orders/{NIL}/start")),
        ("GET", format!("/quotes/{NIL}/repair-order")),
        ("PATCH", format!("/quotes/{NIL}/repair-order")),
        ("DELETE", format!("/quotes/{NIL}/repair-order")),
    ] {
        let (status, _) = call(method, &uri, Some(KEY), Some("{}")).await;
        assert!(
            status == StatusCode::METHOD_NOT_ALLOWED || status == StatusCode::NOT_FOUND,
            "{method} {uri} gave {status}"
        );
    }
}

#[tokio::test]
async fn an_id_that_is_not_a_uuid_is_a_repair_order_nobody_has() {
    for (method, uri, body) in [
        ("GET", "/repair-orders/nope", None),
        ("PATCH", "/repair-orders/nope", Some(r#"{"due_date":null}"#)),
        ("POST", "/repair-orders/nope/start", None),
        ("POST", "/repair-orders/nope/complete", None),
        (
            "POST",
            "/repair-orders/nope/cancel",
            Some(r#"{"reason":"no"}"#),
        ),
    ] {
        assert_eq!(
            refused(method, uri, body).await,
            (StatusCode::NOT_FOUND, "UNKNOWN_REPAIR_ORDER".to_string()),
            "{method} {uri}"
        );
    }
    // and an id that is not a UUID is a quote nobody has
    assert_eq!(
        refused("POST", "/quotes/nope/repair-order", Some("{}")).await,
        (StatusCode::NOT_FOUND, "UNKNOWN_QUOTE".to_string())
    );
}

#[tokio::test]
async fn a_bad_list_query_is_refused() {
    for query in [
        "customer_id=nope",
        "branch_id=nope",
        "equipment_id=1",
        "status=done",
        "status=Open",
        "status=",
        "limit=many",
        "offset=-1",
    ] {
        assert_eq!(
            refused("GET", &format!("/repair-orders?{query}"), None).await,
            (StatusCode::BAD_REQUEST, "INVALID_QUERY".to_string()),
            "{query}"
        );
    }
}

#[tokio::test]
async fn opening_a_repair_order_with_a_wrong_body_is_refused_before_anything_is_saved() {
    for body in [
        "not json",
        "[]",
        r#"{"meter_reading":"1250"}"#,
        r#"{"meter_reading":true}"#,
        // things a caller must not choose
        r#"{"status":"completed"}"#,
        r#"{"number":"RO-2026-00000001"}"#,
        r#"{"quote_id":"x"}"#,
        r#"{"meter_reading":5,"due_date":"2030-01-01"}"#,
    ] {
        assert_eq!(
            refused("POST", &format!("/quotes/{NIL}/repair-order"), Some(body)).await,
            (StatusCode::BAD_REQUEST, "INVALID_REQUEST_BODY".to_string()),
            "{body}"
        );
    }
    // a call with no body at all is wrong too (the body is required: `{}` is fine)
    assert_eq!(
        refused("POST", &format!("/quotes/{NIL}/repair-order"), None)
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn a_repair_order_change_or_cancel_with_a_wrong_body_is_refused() {
    for body in [
        "{}",
        "[]",
        "not json",
        r#"{"due_date":5}"#,
        r#"{"due_date":"tomorrow"}"#,
        r#"{"due_date":"2030-02-30"}"#,
        r#"{"due_date":"2030-01-01","status":"completed"}"#,
        // the paragraphs are text or null, nothing else
        r#"{"complaint":5}"#,
        r#"{"cause":true}"#,
        r#"{"correction":["a"]}"#,
        r#"{"complaint":{"text":"a"}}"#,
        r#"{"complaint":"ok","note":"unknown field"}"#,
        r#"{"due_date":"2030-01-01","complaint":5}"#,
        r#"{"status":"completed"}"#,
        r#"{"started_at":"2030-01-01T00:00:00"}"#,
    ] {
        assert_eq!(
            refused("PATCH", &format!("/repair-orders/{NIL}"), Some(body)).await,
            (StatusCode::BAD_REQUEST, "INVALID_REQUEST_BODY".to_string()),
            "{body}"
        );
    }
    for body in [
        "{}",
        "not json",
        r#"{"reason":5}"#,
        r#"{"reason":null}"#,
        r#"{"reason":"no","status":"open"}"#,
    ] {
        assert_eq!(
            refused("POST", &format!("/repair-orders/{NIL}/cancel"), Some(body)).await,
            (StatusCode::BAD_REQUEST, "INVALID_REQUEST_BODY".to_string()),
            "{body}"
        );
    }
}

#[tokio::test]
async fn a_bad_if_match_is_refused_before_the_database_is_touched_on_every_changing_call() {
    let calls = [
        (
            "PATCH",
            format!("/repair-orders/{NIL}"),
            Some(r#"{"due_date":null}"#),
        ),
        ("POST", format!("/repair-orders/{NIL}/start"), None),
        ("POST", format!("/repair-orders/{NIL}/complete"), None),
        (
            "POST",
            format!("/repair-orders/{NIL}/cancel"),
            Some(r#"{"reason":"no"}"#),
        ),
    ];
    for bad in ["abc", "", "-1", "1.5", "\"x\"", "W/", "99999999999"] {
        for (method, uri, body) in &calls {
            let (status, answer) = call_with_header(method, uri, *body, "if-match", bad).await;
            assert_eq!(
                (status, answer["code"].as_str()),
                (StatusCode::BAD_REQUEST, Some("INVALID_IF_MATCH")),
                "{method} {uri} If-Match: {bad:?}"
            );
        }
    }
}
