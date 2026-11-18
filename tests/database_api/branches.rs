// tests/database_api/branches.rs: the branch calls with a real database behind them

use super::support::{KEY, branch_input, setup};
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_created_branch_is_active_and_comes_back_cleaned_up() {
    let api = setup().await;
    let (status, body) = api
        .call("POST", "/branches", Some(branch_input("API-T-01")))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["status"], "active");
    assert_eq!(body["number"], "API-T-01");
    assert_eq!(body["postal_code"], "S4P 3Y2"); // tidied by the library
    assert_eq!(body["phone"], "+1-306-555-0100");
    assert_eq!(body["province"], "SK");
    assert_eq!(body["country"], "CA");
    assert_eq!(body["address_line2"], "Unit 4");
    assert_eq!(body["latitude"], 50.4452);
    assert_eq!(body["id"].as_str().unwrap().len(), 36);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_branch_is_read_back_by_its_id() {
    let api = setup().await;
    let id = api.create_branch("API-T-02").await;
    let (status, body) = api.call("GET", &format!("/branches/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["number"], "API-T-02");
    assert_eq!(body["id"], id);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_list_can_show_only_active_or_only_inactive_branches() {
    let api = setup().await;
    api.create_branch("API-T-03").await;
    let closing = api.create_branch("API-T-04").await;
    api.call(
        "PATCH",
        &format!("/branches/{closing}/status"),
        Some(json!({"status": "inactive"})),
    )
    .await;

    let numbers = |body: &Value| -> Vec<String> {
        body.as_array()
            .unwrap()
            .iter()
            .map(|b| b["number"].as_str().unwrap().to_string())
            .filter(|n| n.starts_with("API-T-"))
            .collect()
    };
    let (_, all) = api.call("GET", "/branches", None).await;
    let (_, active) = api.call("GET", "/branches?status=active", None).await;
    let (_, inactive) = api.call("GET", "/branches?status=inactive", None).await;
    assert_eq!(numbers(&all), ["API-T-03", "API-T-04"]);
    assert_eq!(numbers(&active), ["API-T-03"]);
    assert_eq!(numbers(&inactive), ["API-T-04"]);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_status_is_switched_both_ways_and_nothing_else_changes() {
    let api = setup().await;
    let id = api.create_branch("API-T-05").await;
    let uri = format!("/branches/{id}/status");

    let (status, off) = api
        .call("PATCH", &uri, Some(json!({"status": "inactive"})))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(off["status"], "inactive");
    assert_eq!(off["name"], "Test Branch");

    let (_, on) = api
        .call("PATCH", &uri, Some(json!({"status": "active"})))
        .await;
    assert_eq!(on["status"], "active");
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn replacing_the_details_keeps_an_inactive_branch_inactive() {
    let api = setup().await;
    let id = api.create_branch("API-T-06").await;
    api.call(
        "PATCH",
        &format!("/branches/{id}/status"),
        Some(json!({"status": "inactive"})),
    )
    .await;

    let mut changed = branch_input("API-T-06");
    changed["name"] = json!("Renamed");
    changed["province"] = json!("AB");
    changed["phone"] = json!("403-555-0199");
    changed.as_object_mut().unwrap().remove("latitude");
    changed.as_object_mut().unwrap().remove("longitude");
    let (status, body) = api
        .call("PUT", &format!("/branches/{id}"), Some(changed))
        .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["name"], "Renamed");
    assert_eq!(body["province"], "AB");
    assert_eq!(body["latitude"], Value::Null);
    assert_eq!(body["id"], id);
    assert_eq!(body["status"], "inactive"); // not reopened by accident
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn two_branches_cannot_share_a_number() {
    let api = setup().await;
    api.create_branch("API-T-07").await;
    let other = api.create_branch("API-T-08").await;

    let (status, body) = api
        .call("POST", "/branches", Some(branch_input("API-T-07")))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "DUPLICATE_BRANCH_NUMBER");

    let (status, _) = api
        .call(
            "PUT",
            &format!("/branches/{other}"),
            Some(branch_input("API-T-07")),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_branch_nobody_has_is_not_found() {
    let api = setup().await;
    let nobody = "00000000-0000-0000-0000-000000000000";
    for (method, uri, body) in [
        ("GET", format!("/branches/{nobody}"), None),
        (
            "PUT",
            format!("/branches/{nobody}"),
            Some(branch_input("API-T-09")),
        ),
        (
            "PATCH",
            format!("/branches/{nobody}/status"),
            Some(json!({"status": "inactive"})),
        ),
    ] {
        let (status, answer) = api.call(method, &uri, body).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method}");
        assert_eq!(answer["code"], "UNKNOWN_BRANCH");
    }
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_database_that_is_down_gives_a_plain_503_with_no_details() {
    let api = setup().await;
    // Close the connection pool: from now on every query fails, like a database that went away.
    api.db.clone().close().await.unwrap();
    let request = Request::builder()
        .uri("/branches")
        .header(header::AUTHORIZATION, format!("Bearer {KEY}"))
        .body(Body::empty())
        .unwrap();
    let response = api.app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let bytes = to_bytes(response.into_body(), 1 << 20).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["code"], "DATABASE_UNAVAILABLE");
}
