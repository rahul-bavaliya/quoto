// tests/api/quotes.rs: the quote calls that are answered before the database is touched

use super::support::{KEY, call, call_with_header};
use axum::http::StatusCode;

const NIL: &str = "00000000-0000-0000-0000-000000000000";

fn line(kind: &str) -> String {
    format!(
        r#"{{"description":"Oil filter","quantity":1,"unit_price_cents":1500,"kind":"{kind}"}}"#
    )
}

async fn refused(method: &str, uri: &str, body: Option<&str>) -> (StatusCode, String) {
    let (status, answer) = call(method, uri, Some(KEY), body).await;
    (status, answer["code"].as_str().unwrap_or("?").to_string())
}

#[tokio::test]
async fn every_quote_call_needs_the_key() {
    for (method, uri) in [
        ("GET", "/quotes".to_string()),
        ("POST", "/quotes".to_string()),
        ("GET", format!("/quotes/{NIL}")),
        ("GET", format!("/quotes/{NIL}/pdf")),
        ("PATCH", format!("/quotes/{NIL}")),
        ("POST", format!("/quotes/{NIL}/lines")),
        ("PATCH", format!("/quotes/{NIL}/customer")),
        ("PATCH", format!("/quotes/{NIL}/equipment")),
        ("POST", format!("/quotes/{NIL}/accept")),
        ("POST", format!("/quotes/{NIL}/reject")),
        ("POST", format!("/quotes/{NIL}/reopen")),
    ] {
        let (status, _) = call(method, &uri, None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}");
    }
}

#[tokio::test]
async fn a_quote_cannot_be_deleted_or_replaced_or_given_a_status() {
    for (method, uri) in [
        ("DELETE", format!("/quotes/{NIL}")),
        ("PUT", format!("/quotes/{NIL}")),
        ("PATCH", format!("/quotes/{NIL}/status")),
        ("DELETE", "/quotes".to_string()),
        ("PUT", "/quotes".to_string()),
        ("GET", format!("/quotes/{NIL}/accept")),
        ("POST", format!("/quotes/{NIL}/pdf")),
        ("DELETE", format!("/quotes/{NIL}/pdf")),
    ] {
        let (status, _) = call(method, &uri, Some(KEY), Some("{}")).await;
        assert!(
            status == StatusCode::METHOD_NOT_ALLOWED || status == StatusCode::NOT_FOUND,
            "{method} {uri} gave {status}"
        );
    }
}

#[tokio::test]
async fn an_id_that_is_not_a_uuid_is_a_quote_nobody_has() {
    let good_line = line("part");
    for (method, uri, body) in [
        ("GET", "/quotes/nope", None),
        ("GET", "/quotes/nope/pdf", None),
        (
            "PATCH",
            "/quotes/nope",
            Some(r#"{"expires_on":"2030-01-01"}"#),
        ),
        ("POST", "/quotes/nope/lines", Some(good_line.as_str())),
        (
            "PATCH",
            "/quotes/nope/customer",
            Some(&format!(r#"{{"customer_id":"{NIL}"}}"#)[..]),
        ),
        (
            "PATCH",
            "/quotes/nope/equipment",
            Some(&format!(r#"{{"equipment_id":"{NIL}"}}"#)[..]),
        ),
        ("POST", "/quotes/nope/accept", None),
        ("POST", "/quotes/nope/reject", Some(r#"{"reason":"no"}"#)),
        ("POST", "/quotes/nope/reopen", None),
    ] {
        assert_eq!(
            refused(method, uri, body).await,
            (StatusCode::NOT_FOUND, "UNKNOWN_QUOTE".to_string()),
            "{method} {uri}"
        );
    }
}

#[tokio::test]
async fn a_bad_list_query_is_refused() {
    for query in [
        "customer_id=nope",
        "branch_id=nope",
        "equipment_id=1",
        "limit=many",
        "offset=-1",
    ] {
        assert_eq!(
            refused("GET", &format!("/quotes?{query}"), None).await,
            (StatusCode::BAD_REQUEST, "INVALID_QUERY".to_string()),
            "{query}"
        );
    }
}

#[tokio::test]
async fn a_new_quote_with_a_wrong_body_is_refused_before_anything_is_saved() {
    let wrong_bodies = [
        "not json",
        "[]",
        "{}",
        r#"{"branch_id":5}"#,
        r#"{"branch_id":null}"#,
        // fields a caller must not choose
        &format!(r#"{{"branch_id":"{NIL}","status":"accepted"}}"#),
        &format!(r#"{{"branch_id":"{NIL}","number":"Q-2026-00000001"}}"#),
        &format!(r#"{{"branch_id":"{NIL}","id":"{NIL}"}}"#),
        &format!(r#"{{"branch_id":"{NIL}","created_at":"2020-01-01T00:00:00"}}"#),
        &format!(r#"{{"branch_id":"{NIL}","items":"none"}}"#),
        &format!(r#"{{"branch_id":"{NIL}","items":[5]}}"#),
        &format!(r#"{{"branch_id":"{NIL}","customer_id":5}}"#),
    ];
    for body in wrong_bodies {
        assert_eq!(
            refused("POST", "/quotes", Some(body)).await,
            (StatusCode::BAD_REQUEST, "INVALID_REQUEST_BODY".to_string()),
            "{body}"
        );
    }
}

#[tokio::test]
async fn an_id_in_a_new_quote_that_is_not_a_uuid_names_nobody() {
    for (body, code) in [
        (r#"{"branch_id":"nope"}"#.to_string(), "UNKNOWN_BRANCH"),
        (
            format!(r#"{{"branch_id":"{NIL}","customer_id":"nope"}}"#),
            "UNKNOWN_CUSTOMER",
        ),
        (
            format!(r#"{{"branch_id":"{NIL}","equipment_id":"nope"}}"#),
            "UNKNOWN_EQUIPMENT",
        ),
    ] {
        assert_eq!(
            refused("POST", "/quotes", Some(&body)).await,
            (StatusCode::NOT_FOUND, code.to_string()),
            "{body}"
        );
    }
}

fn bad_lines() -> Vec<(String, &'static str)> {
    let with = |field: &str, value: &str| {
        let mut text = line("part");
        let start = text.find(&format!("\"{field}\":")).unwrap();
        let end = text[start..].find([',', '}']).map(|at| start + at).unwrap();
        text.replace_range(start..end, &format!("\"{field}\":{value}"));
        text
    };
    vec![
        (line("gift"), "INVALID_LINE_KIND"),
        (line(""), "INVALID_LINE_KIND"),
        (with("quantity", "0"), "BAD_QUANTITY"),
        (with("quantity", "-2"), "BAD_QUANTITY"),
        (with("quantity", "1000001"), "LINE_TOO_LARGE"),
        (with("unit_price_cents", "-1"), "NEGATIVE_PRICE"),
        (with("unit_price_cents", "100000001"), "LINE_TOO_LARGE"),
        (with("description", "\"   \""), "MISSING_FIELD"),
        (
            with("description", &format!("\"{}\"", "x".repeat(201))),
            "DESCRIPTION_TOO_LONG",
        ),
    ]
}

#[tokio::test]
async fn a_wrong_line_is_refused_with_the_rule_that_refused_it_before_the_database_is_touched() {
    for (bad, code) in bad_lines() {
        // as part of a new quote...
        let new = format!(r#"{{"branch_id":"{NIL}","items":[{bad}]}}"#);
        let (status, got) = refused("POST", "/quotes", Some(&new)).await;
        assert_eq!(
            (status, got.as_str()),
            (StatusCode::BAD_REQUEST, code),
            "{bad}"
        );
        // ...and on its own (the id is not a quote, so check the line against a valid-looking id)
        // lines are checked after the quote is read, so only the body shape is tested here
    }
}

#[tokio::test]
async fn a_line_with_the_wrong_shape_is_refused_before_the_database_is_touched() {
    for body in [
        "not json",
        "{}",
        r#"{"description":"x","quantity":1,"unit_price_cents":5}"#,
        r#"{"description":"x","quantity":"1","unit_price_cents":5,"kind":"part"}"#,
        r#"{"description":"x","quantity":1,"unit_price_cents":5.5,"kind":"part"}"#,
        r#"{"description":"x","quantity":1,"unit_price_cents":5,"kind":"part","id":"a"}"#,
        r#"{"description":"x","quantity":1,"unit_price_cents":5,"kind":"part","line_total_cents":5}"#,
    ] {
        assert_eq!(
            refused("POST", &format!("/quotes/{NIL}/lines"), Some(body)).await,
            (StatusCode::BAD_REQUEST, "INVALID_REQUEST_BODY".to_string()),
            "{body}"
        );
    }
}

#[tokio::test]
async fn the_dates_call_wants_at_least_one_real_date_and_nothing_else() {
    for body in [
        "{}",
        "not json",
        r#"{"expires_on":null}"#,
        r#"{"promise_date":null}"#,
        r#"{"expires_on":null,"promise_date":"2030-01-01"}"#,
        r#"{"expires_on":"tomorrow"}"#,
        r#"{"expires_on":"2030-13-45"}"#,
        r#"{"expires_on":20300101}"#,
        r#"{"status":"accepted"}"#,
        r#"{"id":"x"}"#,
        r#"{"expires_on":"2030-01-01","number":"Q-2030-00000001"}"#,
        r#"{"expires_on":"2030-01-01","created_at":"2020-01-01"}"#,
        r#"{"customer_id":"x"}"#,
    ] {
        assert_eq!(
            refused("PATCH", &format!("/quotes/{NIL}"), Some(body)).await,
            (StatusCode::BAD_REQUEST, "INVALID_REQUEST_BODY".to_string()),
            "{body}"
        );
    }
}

#[tokio::test]
async fn adding_a_customer_or_a_machine_wants_exactly_one_key_holding_an_id() {
    for (what, other) in [("customer", "equipment"), ("equipment", "customer")] {
        let uri = format!("/quotes/{NIL}/{what}");
        for body in [
            "{}".to_string(),
            "not json".to_string(),
            format!(r#"{{"{what}_id":null}}"#),
            format!(r#"{{"{what}_id":5}}"#),
            format!(r#"{{"{other}_id":"{NIL}"}}"#),
            format!(r#"{{"{what}_id":"{NIL}","{other}_id":"{NIL}"}}"#),
            format!(r#"{{"{what}_id":"{NIL}","status":"x"}}"#),
        ] {
            assert_eq!(
                refused("PATCH", &uri, Some(&body)).await,
                (StatusCode::BAD_REQUEST, "INVALID_REQUEST_BODY".to_string()),
                "{what}: {body}"
            );
        }
    }
    // an id that is not a UUID names nobody
    assert_eq!(
        refused(
            "PATCH",
            &format!("/quotes/{NIL}/customer"),
            Some(r#"{"customer_id":"nope"}"#)
        )
        .await,
        (StatusCode::NOT_FOUND, "UNKNOWN_CUSTOMER".to_string())
    );
    assert_eq!(
        refused(
            "PATCH",
            &format!("/quotes/{NIL}/equipment"),
            Some(r#"{"equipment_id":"nope"}"#)
        )
        .await,
        (StatusCode::NOT_FOUND, "UNKNOWN_EQUIPMENT".to_string())
    );
}

#[tokio::test]
async fn a_rejection_needs_a_reason_and_nothing_else() {
    for body in [
        "{}",
        "not json",
        r#"{"reason":null}"#,
        r#"{"reason":5}"#,
        r#"{"why":"x"}"#,
        r#"{"reason":"x","status":"rejected"}"#,
    ] {
        assert_eq!(
            refused("POST", &format!("/quotes/{NIL}/reject"), Some(body)).await,
            (StatusCode::BAD_REQUEST, "INVALID_REQUEST_BODY".to_string()),
            "{body}"
        );
    }
}

#[tokio::test]
async fn a_big_body_is_refused() {
    let big = format!(
        r#"{{"branch_id":"{NIL}","items":[{{"description":"{}","quantity":1,"unit_price_cents":1,"kind":"part"}}]}}"#,
        "x".repeat(20_000)
    );
    let (status, _) = call("POST", "/quotes", Some(KEY), Some(&big)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST); // like every call: one plain answer for a body that is not accepted
}

#[tokio::test]
async fn a_bad_if_match_is_refused_before_the_database_is_touched_on_every_changing_call() {
    let one_line = line("part");
    let customer = format!(r#"{{"customer_id":"{NIL}"}}"#);
    let equipment = format!(r#"{{"equipment_id":"{NIL}"}}"#);
    let calls = [
        (
            "PATCH",
            format!("/quotes/{NIL}"),
            Some(r#"{"expires_on":"2099-01-01"}"#),
        ),
        (
            "POST",
            format!("/quotes/{NIL}/lines"),
            Some(one_line.as_str()),
        ),
        (
            "PATCH",
            format!("/quotes/{NIL}/customer"),
            Some(customer.as_str()),
        ),
        (
            "PATCH",
            format!("/quotes/{NIL}/equipment"),
            Some(equipment.as_str()),
        ),
        ("POST", format!("/quotes/{NIL}/accept"), None),
        (
            "POST",
            format!("/quotes/{NIL}/reject"),
            Some(r#"{"reason":"no"}"#),
        ),
        ("POST", format!("/quotes/{NIL}/reopen"), None),
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
