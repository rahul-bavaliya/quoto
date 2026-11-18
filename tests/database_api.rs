// tests/database_api.rs: the branch API with a real database behind it
//
// Needs a running database, so these are skipped normally (and in CI). Run them by hand:
//
//     docker compose up -d
//     cargo test --test database_api -- --ignored
//
// The API commits each call for real (that is what it does), and a branch can never be deleted
// through it. So every test branch has a number starting with "API-T-", and the tests remove
// those rows themselves (with SQL, as test housekeeping) before and after each test.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use quoto::api::router;
use quoto::*;
use sea_orm::{ConnectionTrait, DatabaseConnection};
use serde_json::{Value, json};
use tower::ServiceExt;

const KEY: &str = "test-key-test-key-test-key-test-key-1234";

/// The tests take turns: they share one table and clean it.
static ONE_AT_A_TIME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct Api {
    app: Router,
    db: DatabaseConnection,
    _turn: tokio::sync::MutexGuard<'static, ()>,
}

async fn clean(db: &DatabaseConnection) {
    db.execute_unprepared("DELETE FROM branches WHERE number LIKE 'API-T-%'")
        .await
        .expect("clean up test branches");
}

async fn setup() -> Api {
    let turn = ONE_AT_A_TIME.lock().await;
    dotenvy::dotenv().ok();
    let settings = DatabaseSettings::from_env().expect("database settings (see .env.example)");
    let db = database::connect(&settings).await.expect("database");
    database::sync(&db).await.expect("sync");
    clean(&db).await;
    Api {
        app: router(db.clone(), Secret::new(KEY.to_string())),
        db,
        _turn: turn,
    }
}

impl Api {
    async fn call(&self, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
        let mut request = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::AUTHORIZATION, format!("Bearer {KEY}"));
        let text = match &body {
            Some(value) => {
                request = request.header(header::CONTENT_TYPE, "application/json");
                value.to_string()
            }
            None => String::new(),
        };
        let response = self
            .app
            .clone()
            .oneshot(request.body(Body::from(text)).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 1 << 20).await.unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    /// Creates a branch and returns its id.
    async fn create(&self, number: &str) -> String {
        let (status, body) = self.call("POST", "/branches", Some(input(number))).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body["id"].as_str().unwrap().to_string()
    }

    async fn finish(self) {
        clean(&self.db).await;
    }
}

fn input(number: &str) -> Value {
    json!({
        "number": number, "name": "Test Branch",
        "address_line1": "1 Industrial Rd", "address_line2": "Unit 4", "city": "Regina",
        "province": "SK", "postal_code": "s4p3y2", "phone": "306-555-0100",
        "latitude": 50.4452, "longitude": -104.6189
    })
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_created_branch_is_active_and_comes_back_cleaned_up() {
    let api = setup().await;
    let (status, body) = api.call("POST", "/branches", Some(input("API-T-01"))).await;
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
    let id = api.create("API-T-02").await;
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
    api.create("API-T-03").await;
    let closing = api.create("API-T-04").await;
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
    let id = api.create("API-T-05").await;
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
    let id = api.create("API-T-06").await;
    api.call(
        "PATCH",
        &format!("/branches/{id}/status"),
        Some(json!({"status": "inactive"})),
    )
    .await;

    let mut changed = input("API-T-06");
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
    api.create("API-T-07").await;
    let other = api.create("API-T-08").await;

    let (status, body) = api.call("POST", "/branches", Some(input("API-T-07"))).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "DUPLICATE_BRANCH_NUMBER");

    let (status, _) = api
        .call(
            "PUT",
            &format!("/branches/{other}"),
            Some(input("API-T-07")),
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
            Some(input("API-T-09")),
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
