// tests/api/equipment.rs: the equipment calls that are answered before the database is touched

use super::support::{KEY, call};
use axum::http::StatusCode;

const NIL: &str = "00000000-0000-0000-0000-000000000000";

const GOOD: &str = r#"{"make":"John Deere","model":"8R 310","meter_type":"hr","equipment_type":"tractor","serial_number":"T-1"}"#;

#[tokio::test]
async fn equipment_calls_need_the_key() {
    for (method, uri) in [
        ("GET", "/equipment"),
        ("POST", "/equipment"),
        ("GET", &format!("/equipment/{NIL}")),
        ("DELETE", &format!("/equipment/{NIL}")),
        ("PATCH", &format!("/equipment/{NIL}/owner")),
    ] {
        let (status, _) = call(method, uri, None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}");
    }
}

#[tokio::test]
async fn an_id_that_is_not_a_uuid_is_a_machine_nobody_has() {
    for (method, body) in [
        ("GET", None),
        ("DELETE", None),
        ("PUT", Some(GOOD)),
        ("PATCH", Some(r#"{"plate":"X"}"#)),
    ] {
        let (status, answer) = call(method, "/equipment/nope", Some(KEY), body).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method}");
        assert_eq!(answer["code"], "UNKNOWN_EQUIPMENT", "{method}");
    }
    let (status, answer) = call(
        "PATCH",
        "/equipment/nope/status",
        Some(KEY),
        Some(r#"{"status":"inactive"}"#),
    )
    .await;
    assert_eq!(
        (status, answer["code"].as_str()),
        (StatusCode::NOT_FOUND, Some("UNKNOWN_EQUIPMENT"))
    );
    let (status, answer) = call(
        "PATCH",
        "/equipment/nope/owner",
        Some(KEY),
        Some(r#"{"customer_id":null}"#),
    )
    .await;
    assert_eq!(
        (status, answer["code"].as_str()),
        (StatusCode::NOT_FOUND, Some("UNKNOWN_EQUIPMENT"))
    );
}

#[tokio::test]
async fn a_bad_create_body_is_refused() {
    for body in [
        "",
        "not json",
        "[]",
        "{}",
        // unknown field, and fields that are not details
        r#"{"make":"A","model":"B","meter_type":"hr","equipment_type":"tractor","serial_number":"X","colour":"red"}"#,
        r#"{"make":"A","model":"B","meter_type":"hr","equipment_type":"tractor","serial_number":"X","status":"inactive"}"#,
        r#"{"make":"A","model":"B","meter_type":"hr","equipment_type":"tractor","serial_number":"X","source":"x"}"#,
        r#"{"make":"A","model":"B","meter_type":"hr","equipment_type":"tractor","serial_number":"X","id":"x"}"#,
        // owner must be a string or null
        r#"{"make":"A","model":"B","meter_type":"hr","equipment_type":"tractor","serial_number":"X","customer_id":5}"#,
    ] {
        let (status, answer) = call("POST", "/equipment", Some(KEY), Some(body)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert_eq!(answer["code"], "INVALID_REQUEST_BODY", "{body}");
    }
}

#[tokio::test]
async fn a_rule_that_refuses_a_value_is_a_400_with_its_own_code() {
    for (body, code) in [
        (
            r#"{"make":"A","model":"B","meter_type":"parsecs","equipment_type":"tractor","serial_number":"X"}"#,
            "INVALID_METER_TYPE",
        ),
        (
            r#"{"make":"A","model":"B","meter_type":"hr","equipment_type":"spaceship","serial_number":"X"}"#,
            "INVALID_EQUIPMENT_TYPE",
        ),
        (
            r#"{"make":" ","model":"B","meter_type":"hr","equipment_type":"tractor","serial_number":"X"}"#,
            "",
        ),
        (
            r#"{"make":"A","model":"B","meter_type":"hr","equipment_type":"tractor"}"#,
            "",
        ),
        (
            r#"{"make":"A","model":"B","meter_type":"hr","equipment_type":"tractor","serial_number":"X","meter_reading":-1}"#,
            "",
        ),
    ] {
        let (status, answer) = call("POST", "/equipment", Some(KEY), Some(body)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        if !code.is_empty() {
            assert_eq!(answer["code"], code, "{body}");
        }
    }
}

#[tokio::test]
async fn an_owner_that_is_not_a_uuid_is_a_customer_nobody_has() {
    let body = r#"{"make":"A","model":"B","meter_type":"hr","equipment_type":"tractor","serial_number":"X","customer_id":"nope"}"#;
    let (status, answer) = call("POST", "/equipment", Some(KEY), Some(body)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(answer["code"], "UNKNOWN_CUSTOMER");
}

#[tokio::test]
async fn a_replace_or_patch_cannot_carry_the_owner_the_status_the_source_or_an_id() {
    for field in [
        r#""customer_id":null"#,
        r#""status":"inactive""#,
        r#""source":"x""#,
        r#""id":"x""#,
    ] {
        let put = format!(
            r#"{{"make":"A","model":"B","meter_type":"hr","equipment_type":"tractor","serial_number":"X",{field}}}"#
        );
        let (status, answer) =
            call("PUT", &format!("/equipment/{NIL}"), Some(KEY), Some(&put)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "PUT {field}");
        assert_eq!(answer["code"], "INVALID_REQUEST_BODY");
        let patch = format!("{{{field}}}");
        let (status, answer) = call(
            "PATCH",
            &format!("/equipment/{NIL}"),
            Some(KEY),
            Some(&patch),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "PATCH {field}");
        assert_eq!(answer["code"], "INVALID_REQUEST_BODY");
    }
}

#[tokio::test]
async fn the_owner_call_needs_exactly_one_customer_id_so_nothing_is_cleared_by_accident() {
    for body in [
        "{}",
        "",
        "[]",
        r#"{"customer":null}"#,
        r#"{"customer_id":5}"#,
        r#"{"customer_id":null,"status":"inactive"}"#,
        r#"{"customer_id":"a","extra":1}"#,
    ] {
        let (status, answer) = call(
            "PATCH",
            &format!("/equipment/{NIL}/owner"),
            Some(KEY),
            Some(body),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert_eq!(answer["code"], "INVALID_REQUEST_BODY", "{body}");
    }
    let (status, answer) = call(
        "PATCH",
        &format!("/equipment/{NIL}/owner"),
        Some(KEY),
        Some(r#"{"customer_id":"nope"}"#),
    )
    .await;
    assert_eq!(
        (status, answer["code"].as_str()),
        (StatusCode::NOT_FOUND, Some("UNKNOWN_CUSTOMER"))
    );
}

#[tokio::test]
async fn a_status_call_with_a_bad_status_is_refused() {
    let (status, answer) = call(
        "PATCH",
        &format!("/equipment/{NIL}/status"),
        Some(KEY),
        Some(r#"{"status":"maybe"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(answer["code"], "INVALID_STATUS");
}

#[tokio::test]
async fn a_list_query_that_makes_no_sense_is_refused() {
    for query in [
        "status=maybe",
        "customer_id=nope",
        &format!("customer_id={NIL}&unowned=true"),
        "unowned=perhaps",
        "limit=abc",
    ] {
        let (status, _) = call("GET", &format!("/equipment?{query}"), Some(KEY), None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}");
    }
}
