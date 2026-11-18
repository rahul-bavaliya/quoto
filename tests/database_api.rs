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
use std::io::Write;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use tracing_subscriber::fmt::MakeWriter;

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
    db.execute_unprepared("DELETE FROM customers WHERE name LIKE 'API-T-%'")
        .await
        .expect("clean up test customers");
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

// ===== what the log says about real calls (rules: claude/logging-rules.md) =====

/// A place the log lines go instead of the screen, so a test can read them.
#[derive(Clone, Default)]
struct LogText(Arc<Mutex<Vec<u8>>>);

impl Write for LogText {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for LogText {
    type Writer = LogText;
    fn make_writer(&'a self) -> LogText {
        self.clone()
    }
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn real_calls_log_nothing_about_the_branch_and_a_database_failure_is_an_error_line() {
    let api = setup().await;
    let log = LogText::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(log.clone())
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE) // every level, every crate (sqlx too)
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let mut body = input("API-T-LOG");
    body["name"] = json!("Zed Quiet Branch");
    let (_, created) = api.call("POST", "/branches", Some(body)).await;
    let id = created["id"].as_str().unwrap().to_string();
    api.call("GET", &format!("/branches/{id}"), None).await;
    api.call(
        "PATCH",
        &format!("/branches/{id}/status"),
        Some(json!({"status": "inactive"})),
    )
    .await;
    // Now the database goes away.
    api.db.clone().close().await.unwrap();
    api.call("GET", "/branches", None).await;

    let text = log.0.lock().unwrap().clone();
    let text = String::from_utf8_lossy(&text).to_string();
    for secret in [
        id.as_str(),
        "Zed Quiet Branch",
        "API-T-LOG",
        "1 Industrial Rd",
        "Unit 4",
        "Regina",
        "S4P 3Y2",
        "306-555-0100",
        "50.4452",
        KEY,
    ] {
        assert!(
            !text.contains(secret),
            "{secret} leaked into the log:\n{text}"
        );
    }
    let calls: Vec<&str> = text.lines().filter(|l| l.contains("api_request")).collect();
    assert_eq!(calls.len(), 4, "{text}");
    assert!(
        calls[0].contains("INFO") && calls[0].contains("status=201"),
        "{}",
        calls[0]
    );
    assert!(
        calls[1].contains("route=\"/branches/{id}\""),
        "{}",
        calls[1]
    );
    // Our side failing is the only thing that is an `error`.
    assert!(
        calls[3].contains("ERROR") && calls[3].contains("code=\"DATABASE_UNAVAILABLE\""),
        "{}",
        calls[3]
    );
    // The branch is cleaned up with the others (the pool is closed, so reconnect first).
    let settings = DatabaseSettings::from_env().unwrap();
    clean(&database::connect(&settings).await.unwrap()).await;
}

// ===== customers =====

fn customer_input(name: &str) -> Value {
    json!({
        "name": name, "company": "Prairie Farms Ltd.", "designation": "Service Manager",
        "phone": "306-555-0142", "alternate_phone": "(306) 555-0143", "email": "Jane@Example.COM",
        "address_line1": "123 Main St", "address_line2": "Suite 2", "city": "Regina",
        "province": "SK", "postal_code": "s4p3y2"
    })
}

impl Api {
    async fn create_customer(&self, name: &str) -> String {
        let (status, body) = self
            .call("POST", "/customers", Some(customer_input(name)))
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body["id"].as_str().unwrap().to_string()
    }
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_created_customer_is_active_and_comes_back_cleaned_up() {
    let api = setup().await;
    let (status, body) = api
        .call("POST", "/customers", Some(customer_input("API-T-Jane")))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["status"], "active");
    assert_eq!(body["name"], "API-T-Jane");
    assert_eq!(body["phone"], "+1-306-555-0142"); // tidied by the library
    assert_eq!(body["alternate_phone"], "+1-306-555-0143");
    assert_eq!(body["email"], "jane@example.com");
    assert_eq!(body["postal_code"], "S4P 3Y2");
    assert_eq!(body["province"], "SK");
    assert_eq!(body["country"], "CA");
    assert_eq!(body["company"], "Prairie Farms Ltd.");
    assert_eq!(body["id"].as_str().unwrap().len(), 36);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_customer_with_only_the_required_parts_comes_back_with_nulls() {
    let api = setup().await;
    let body = json!({
        "name": "API-T-Plain", "phone": "403-555-0100", "address_line1": "1 Main St",
        "city": "Calgary", "province": "AB", "postal_code": "T2P 1J9"
    });
    let (status, saved) = api.call("POST", "/customers", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "{saved}");
    for field in [
        "company",
        "designation",
        "alternate_phone",
        "email",
        "address_line2",
    ] {
        assert_eq!(saved[field], Value::Null, "{field}");
    }
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_customer_is_read_back_by_id_and_two_may_share_a_name() {
    let api = setup().await;
    let one = api.create_customer("API-T-Same").await;
    let two = api.create_customer("API-T-Same").await;
    assert_ne!(one, two);
    let (status, body) = api.call("GET", &format!("/customers/{one}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], one);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_customer_list_is_paged_ordered_by_name_and_filtered_by_status() {
    let api = setup().await;
    api.create_customer("API-T-B").await;
    let a = api.create_customer("API-T-A").await;
    api.create_customer("API-T-C").await;
    api.call(
        "PATCH",
        &format!("/customers/{a}/status"),
        Some(json!({"status": "inactive"})),
    )
    .await;

    let names = |body: &Value| -> Vec<String> {
        body.as_array()
            .unwrap()
            .iter()
            .map(|c| c["name"].as_str().unwrap().to_string())
            .filter(|n| n.starts_with("API-T-"))
            .collect()
    };
    let ids = |body: &Value| -> Vec<String> {
        body.as_array()
            .unwrap()
            .iter()
            .map(|c| c["id"].as_str().unwrap().to_string())
            .collect()
    };
    let (_, all) = api.call("GET", "/customers?limit=200", None).await;
    let (_, active) = api
        .call("GET", "/customers?status=active&limit=200", None)
        .await;
    let (_, inactive) = api
        .call("GET", "/customers?status=inactive&limit=200", None)
        .await;
    assert_eq!(names(&all), ["API-T-A", "API-T-B", "API-T-C"]);
    assert_eq!(names(&active), ["API-T-B", "API-T-C"]);
    assert_eq!(names(&inactive), ["API-T-A"]);

    // Pages line up with the whole list.
    let everything = ids(&all);
    let (_, page) = api.call("GET", "/customers?limit=2&offset=1", None).await;
    assert_eq!(ids(&page), everything[1..3]);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_page_is_never_empty_by_mistake_and_never_huge() {
    let api = setup().await;
    api.create_customer("API-T-One").await;
    api.create_customer("API-T-Two").await;
    let (status, zero) = api.call("GET", "/customers?limit=0", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(zero.as_array().unwrap().len(), 1); // 0 is read as 1
    let (_, huge) = api.call("GET", "/customers?limit=1000000", None).await;
    assert!(huge.as_array().unwrap().len() <= 200);
    let (_, default) = api.call("GET", "/customers", None).await;
    assert!(default.as_array().unwrap().len() <= 50);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_customer_status_is_switched_both_ways() {
    let api = setup().await;
    let id = api.create_customer("API-T-Switch").await;
    let uri = format!("/customers/{id}/status");
    let (status, off) = api
        .call("PATCH", &uri, Some(json!({"status": "inactive"})))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(off["status"], "inactive");
    assert_eq!(off["name"], "API-T-Switch");
    let (_, on) = api
        .call("PATCH", &uri, Some(json!({"status": "active"})))
        .await;
    assert_eq!(on["status"], "active");
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn replacing_the_details_keeps_an_inactive_customer_inactive() {
    let api = setup().await;
    let id = api.create_customer("API-T-Closed").await;
    api.call(
        "PATCH",
        &format!("/customers/{id}/status"),
        Some(json!({"status": "inactive"})),
    )
    .await;

    let changed = json!({
        "name": "API-T-Renamed", "phone": "403-555-0199", "address_line1": "9 New Rd",
        "city": "Calgary", "province": "AB", "postal_code": "T2P 1J9"
    });
    let (status, body) = api
        .call("PUT", &format!("/customers/{id}"), Some(changed))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["name"], "API-T-Renamed");
    assert_eq!(body["company"], Value::Null); // what was left out is cleared
    assert_eq!(body["province"], "AB");
    assert_eq!(body["id"], id);
    assert_eq!(body["status"], "inactive"); // not reopened by accident
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_customer_nobody_has_is_not_found() {
    let api = setup().await;
    let nobody = "00000000-0000-0000-0000-000000000000";
    for (method, uri, body) in [
        ("GET", format!("/customers/{nobody}"), None),
        (
            "PUT",
            format!("/customers/{nobody}"),
            Some(customer_input("API-T-Ghost")),
        ),
        (
            "PATCH",
            format!("/customers/{nobody}/status"),
            Some(json!({"status": "inactive"})),
        ),
    ] {
        let (status, answer) = api.call(method, &uri, body).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method}");
        assert_eq!(answer["code"], "UNKNOWN_CUSTOMER");
    }
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn real_customer_calls_log_nothing_about_the_customer() {
    let api = setup().await;
    let log = LogText::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(log.clone())
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE) // every level, every crate (sqlx too)
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let (_, created) = api
        .call(
            "POST",
            "/customers",
            Some(customer_input("API-T-Zed Quiet Person")),
        )
        .await;
    let id = created["id"].as_str().unwrap().to_string();
    api.call("GET", &format!("/customers/{id}"), None).await;
    api.call("GET", "/customers?status=active&limit=5", None)
        .await;
    api.call(
        "PUT",
        &format!("/customers/{id}"),
        Some(customer_input("API-T-Zed Quiet Person")),
    )
    .await;
    api.call(
        "PATCH",
        &format!("/customers/{id}/status"),
        Some(json!({"status": "inactive"})),
    )
    .await;

    let text = String::from_utf8_lossy(&log.0.lock().unwrap().clone()).to_string();
    for secret in [
        id.as_str(),
        "Zed Quiet Person",
        "Prairie Farms",
        "Service Manager",
        "306-555-0142",
        "555-0143",
        "jane@example.com",
        "Jane@Example",
        "123 Main St",
        "Suite 2",
        "Regina",
        "S4P 3Y2",
        "s4p3y2",
        KEY,
    ] {
        assert!(
            !text.contains(secret),
            "{secret} leaked into the log:\n{text}"
        );
    }
    let calls: Vec<&str> = text.lines().filter(|l| l.contains("api_request")).collect();
    assert_eq!(calls.len(), 5, "{text}");
    assert!(
        calls[1].contains("route=\"/customers/{id}\""),
        "{}",
        calls[1]
    );
    api.finish().await;
}

// ===== PATCH: only the fields that are sent change =====

/// The fields of a saved record that a PATCH may touch (everything but the id and the status).
fn without_status(mut record: Value) -> Value {
    record.as_object_mut().unwrap().remove("status");
    record
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_branch_patch_changes_only_the_sent_field() {
    let api = setup().await;
    let id = api.create("API-T-P1").await;
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
    let id = api.create("API-T-P2").await;
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
    let id = api.create("API-T-P3").await;
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
    let id = api.create("API-T-P4").await;
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
    let id = api.create("API-T-P5").await;
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
    let id = api.create("API-T-P6").await;
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
    let id = api.create("API-T-P7").await;
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
    api.create("API-T-P8").await;
    let other = api.create("API-T-P9").await;
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

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn patch_calls_log_nothing_about_what_was_sent() {
    let api = setup().await;
    let branch = api.create("API-T-PL").await;
    let customer = api.create_customer("API-T-Patch-Log").await;
    let log = LogText::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(log.clone())
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    api.call(
        "PATCH",
        &format!("/customers/{customer}"),
        Some(json!({"name": "API-T-Zed Hidden Person", "email": "hidden.person@secret-mail.example", "phone": "306-555-0177"})),
    )
    .await;
    api.call(
        "PATCH",
        &format!("/branches/{branch}"),
        Some(json!({"name": "Hidden Branch Name", "address_line1": "77 Secret Lane"})),
    )
    .await;
    api.call(
        "PATCH",
        &format!("/customers/{customer}"),
        Some(json!({"phone": "12345"})),
    )
    .await;

    let text = String::from_utf8_lossy(&log.0.lock().unwrap().clone()).to_string();
    for secret in [
        customer.as_str(),
        branch.as_str(),
        "Zed Hidden Person",
        "hidden.person@secret-mail.example",
        "555-0177",
        "Hidden Branch Name",
        "77 Secret Lane",
        "12345",
        KEY,
    ] {
        assert!(
            !text.contains(secret),
            "{secret} leaked into the log:\n{text}"
        );
    }
    let calls: Vec<&str> = text.lines().filter(|l| l.contains("api_request")).collect();
    assert_eq!(calls.len(), 3, "{text}");
    assert!(calls[0].contains("method=PATCH") && calls[0].contains("route=\"/customers/{id}\""));
    assert!(calls[1].contains("route=\"/branches/{id}\""));
    assert!(calls[2].contains("status=400"));
    api.finish().await;
}
