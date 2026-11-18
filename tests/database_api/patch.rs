// tests/database_api/patch.rs: PATCH on branches and customers: only the fields that are sent change

use super::support::setup;
use axum::http::StatusCode;
use serde_json::{Value, json};

/// The fields of a saved record that a PATCH may touch (everything but the id and the status).
fn without_status(mut record: Value) -> Value {
    record.as_object_mut().unwrap().remove("status");
    record
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_branch_patch_changes_only_the_sent_field() {
    let api = setup().await;
    let id = api.create_branch("API-T-P1").await;
    let uri = format!("/branches/{id}");
    let (_, before) = api.call("GET", &uri, None).await;

    let (status, after) = api
        .call("PATCH", &uri, Some(json!({"phone": "(306) 555-0199"})))
        .await;
    assert_eq!(status, StatusCode::OK, "{after}");
    assert_eq!(after["phone"], "+1-306-555-0199"); // tidied like any phone
    let mut expected = before.clone();
    expected["phone"] = json!("+1-306-555-0199");
    assert_eq!(after, expected); // nothing else moved

    // It is really saved, not just echoed.
    let (_, read) = api.call("GET", &uri, None).await;
    assert_eq!(read, expected);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_branch_patch_can_change_several_fields_at_once() {
    let api = setup().await;
    let id = api.create_branch("API-T-P2").await;
    let (status, after) = api
        .call(
            "PATCH",
            &format!("/branches/{id}"),
            Some(json!({"name": "Renamed", "city": "Saskatoon", "province": "AB", "postal_code": "t2p1j9"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{after}");
    assert_eq!(after["name"], "Renamed");
    assert_eq!(after["city"], "Saskatoon");
    assert_eq!(after["province"], "AB");
    assert_eq!(after["postal_code"], "T2P 1J9");
    assert_eq!(after["number"], "API-T-P2"); // not sent, not changed
    assert_eq!(after["address_line1"], "1 Industrial Rd");
    assert_eq!(after["id"], id);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_branch_patch_with_null_clears_an_optional_field_and_only_that_one() {
    let api = setup().await;
    let id = api.create_branch("API-T-P3").await;
    let uri = format!("/branches/{id}");
    let (status, after) = api
        .call("PATCH", &uri, Some(json!({"address_line2": null})))
        .await;
    assert_eq!(status, StatusCode::OK, "{after}");
    assert_eq!(after["address_line2"], Value::Null);
    assert_eq!(after["latitude"], 50.4452); // the map location stays

    // Both halves of the location are cleared together.
    let (status, after) = api
        .call(
            "PATCH",
            &uri,
            Some(json!({"latitude": null, "longitude": null})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{after}");
    assert_eq!(after["latitude"], Value::Null);
    assert_eq!(after["longitude"], Value::Null);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_branch_patch_that_would_leave_half_a_location_is_refused_and_changes_nothing() {
    let api = setup().await;
    let id = api.create_branch("API-T-P4").await;
    let uri = format!("/branches/{id}");
    let (_, before) = api.call("GET", &uri, None).await;
    let (status, answer) = api
        .call(
            "PATCH",
            &uri,
            Some(json!({"latitude": null, "city": "Elsewhere"})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(answer["code"], "INVALID_LOCATION");
    let (_, after) = api.call("GET", &uri, None).await;
    assert_eq!(after, before); // the city did not change either
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_patch_cannot_clear_a_required_field_or_set_what_is_not_an_input() {
    let api = setup().await;
    let id = api.create_branch("API-T-P5").await;
    let uri = format!("/branches/{id}");
    let (_, before) = api.call("GET", &uri, None).await;
    for body in [
        json!({"name": null}),
        json!({"phone": null}),
        json!({"province": null}),
        json!({"name": 5}),
        json!({"status": "inactive"}),
        json!({"id": "00000000-0000-0000-0000-000000000000"}),
        json!({"country": "US"}),
        json!({"nonsense": 1}),
    ] {
        let (status, answer) = api.call("PATCH", &uri, Some(body.clone())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert_eq!(answer["code"], "INVALID_REQUEST_BODY", "{body}");
    }
    let (_, after) = api.call("GET", &uri, None).await;
    assert_eq!(after, before);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_patch_value_a_rule_refuses_changes_nothing() {
    let api = setup().await;
    let id = api.create_branch("API-T-P6").await;
    let uri = format!("/branches/{id}");
    let (_, before) = api.call("GET", &uri, None).await;
    for (body, code) in [
        (
            json!({"phone": "12345", "name": "Changed"}),
            "INVALID_PHONE",
        ),
        (json!({"postal_code": "nope"}), "INVALID_POSTAL_CODE"),
        (json!({"province": "ON"}), "UNKNOWN_PROVINCE"),
        (json!({"name": "   "}), "MISSING_FIELD"),
    ] {
        let (status, answer) = api.call("PATCH", &uri, Some(body.clone())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body} {answer}");
        assert_eq!(answer["code"], code, "{body}");
    }
    let (_, after) = api.call("GET", &uri, None).await;
    assert_eq!(after, before);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_branch_patch_keeps_a_closed_branch_closed() {
    let api = setup().await;
    let id = api.create_branch("API-T-P7").await;
    api.call(
        "PATCH",
        &format!("/branches/{id}/status"),
        Some(json!({"status": "inactive"})),
    )
    .await;
    let (status, after) = api
        .call(
            "PATCH",
            &format!("/branches/{id}"),
            Some(json!({"name": "Still closed"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(after["name"], "Still closed");
    assert_eq!(after["status"], "inactive");
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_branch_patch_cannot_take_another_branchs_number() {
    let api = setup().await;
    api.create_branch("API-T-P8").await;
    let other = api.create_branch("API-T-P9").await;
    let uri = format!("/branches/{other}");
    let (status, answer) = api
        .call("PATCH", &uri, Some(json!({"number": "API-T-P8"})))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(answer["code"], "DUPLICATE_BRANCH_NUMBER");
    // Its own number, sent again, is fine.
    let (status, _) = api
        .call("PATCH", &uri, Some(json!({"number": "API-T-P9"})))
        .await;
    assert_eq!(status, StatusCode::OK);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_patch_for_nobody_is_not_found() {
    let api = setup().await;
    let nobody = "00000000-0000-0000-0000-000000000000";
    let (status, answer) = api
        .call(
            "PATCH",
            &format!("/branches/{nobody}"),
            Some(json!({"name": "x"})),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(answer["code"], "UNKNOWN_BRANCH");
    let (status, answer) = api
        .call(
            "PATCH",
            &format!("/customers/{nobody}"),
            Some(json!({"name": "x"})),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(answer["code"], "UNKNOWN_CUSTOMER");
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_customer_patch_changes_only_the_sent_field() {
    let api = setup().await;
    let id = api.create_customer("API-T-Pat").await;
    let uri = format!("/customers/{id}");
    let (_, before) = api.call("GET", &uri, None).await;
    let (status, after) = api
        .call(
            "PATCH",
            &uri,
            Some(json!({"email": "New.Mail@Example.com"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{after}");
    assert_eq!(after["email"], "new.mail@example.com");
    let mut expected = before.clone();
    expected["email"] = json!("new.mail@example.com");
    assert_eq!(after, expected);
    let (_, read) = api.call("GET", &uri, None).await;
    assert_eq!(read, expected);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_customer_patch_with_null_clears_only_the_optional_fields_named() {
    let api = setup().await;
    let id = api.create_customer("API-T-Clear").await;
    let (status, after) = api
        .call(
            "PATCH",
            &format!("/customers/{id}"),
            Some(json!({"company": null, "email": null})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{after}");
    assert_eq!(after["company"], Value::Null);
    assert_eq!(after["email"], Value::Null);
    assert_eq!(after["designation"], "Service Manager"); // not named, not cleared
    assert_eq!(after["alternate_phone"], "+1-306-555-0143");
    assert_eq!(after["address_line2"], "Suite 2");
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_customer_patch_refuses_what_a_post_would_refuse_and_changes_nothing() {
    let api = setup().await;
    let id = api.create_customer("API-T-Same-Rules").await;
    let uri = format!("/customers/{id}");
    let (_, before) = api.call("GET", &uri, None).await;
    for (body, code) in [
        (json!({"name": null}), "INVALID_REQUEST_BODY"),
        (
            json!({"city": null, "company": "Changed Ltd"}),
            "INVALID_REQUEST_BODY",
        ),
        (json!({"status": "inactive"}), "INVALID_REQUEST_BODY"),
        (json!({"id": "x"}), "INVALID_REQUEST_BODY"),
        (
            json!({"phone": "12345", "company": "Changed Ltd"}),
            "INVALID_PHONE",
        ),
        (json!({"alternate_phone": "12345"}), "INVALID_PHONE"),
        (json!({"email": "not-an-email"}), "INVALID_EMAIL"),
        (json!({"province": "ON"}), "UNKNOWN_PROVINCE"),
    ] {
        let (status, answer) = api.call("PATCH", &uri, Some(body.clone())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body} {answer}");
        assert_eq!(answer["code"], code, "{body}");
    }
    let (_, after) = api.call("GET", &uri, None).await;
    assert_eq!(after, before);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_customer_patch_keeps_an_inactive_customer_inactive() {
    let api = setup().await;
    let id = api.create_customer("API-T-Patch-Closed").await;
    api.call(
        "PATCH",
        &format!("/customers/{id}/status"),
        Some(json!({"status": "inactive"})),
    )
    .await;
    let (status, after) = api
        .call(
            "PATCH",
            &format!("/customers/{id}"),
            Some(json!({"city": "Moose Jaw"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(after["city"], "Moose Jaw");
    assert_eq!(after["status"], "inactive");
    assert_eq!(without_status(after.clone())["name"], "API-T-Patch-Closed");
    api.finish().await;
}
