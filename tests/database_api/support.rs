// tests/database_api/support.rs: what every database API test shares
//
// One `Api` per test: it takes a turn (the tests share the tables), connects, builds the tables,
// cleans the `API-T-` rows (a machine's serial number starts with it too), and makes calls straight into the router (no network). The API commits
// each call for real and nothing can be deleted through it, so `finish` removes the test rows with
// SQL (test housekeeping only).

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

pub const KEY: &str = "test-key-test-key-test-key-test-key-1234";

/// The tests take turns: they share one table and clean it.
static ONE_AT_A_TIME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub struct Api {
    pub app: Router,
    pub db: DatabaseConnection,
    _turn: tokio::sync::MutexGuard<'static, ()>,
}

pub async fn clean(db: &DatabaseConnection) {
    // Quotes first: they point at the test branches, customers and machines (nothing that a quote
    // points at can be deleted). Every test quote is made at an `API-T-` branch.
    db.execute_unprepared(
        "DELETE FROM quote_lines WHERE quote_id IN \
         (SELECT q.id FROM quotes q JOIN branches b ON b.id = q.branch_id WHERE b.number LIKE 'API-T-%')",
    )
    .await
    .expect("clean up test quote lines");
    db.execute_unprepared(
        "DELETE FROM quotes WHERE branch_id IN (SELECT id FROM branches WHERE number LIKE 'API-T-%')",
    )
    .await
    .expect("clean up test quotes");
    // The API dates a quote by the clock, so the tests used up numbers of the real year. Give them
    // back: this year's counter goes back to the highest number a remaining quote has.
    let year = chrono::Local::now().format("%Y").to_string();
    db.execute_unprepared(&format!(
        "UPDATE quote_numbers SET last_number = COALESCE((SELECT max(substring(number from 8)::int) \
         FROM quotes WHERE number LIKE 'Q-{year}-%'), 0) WHERE year = {year}"
    ))
    .await
    .expect("give back the test quote numbers");
    db.execute_unprepared("DELETE FROM branches WHERE number LIKE 'API-T-%'")
        .await
        .expect("clean up test branches");
    db.execute_unprepared("DELETE FROM equipment WHERE serial_number LIKE 'API-T-%'")
        .await
        .expect("clean up test equipment");
    db.execute_unprepared("DELETE FROM customers WHERE name LIKE 'API-T-%'")
        .await
        .expect("clean up test customers");
}

pub async fn setup() -> Api {
    let turn = ONE_AT_A_TIME.lock().await;
    dotenvy::dotenv().ok();
    let settings = DatabaseSettings::from_env().expect("database settings (see .env.example)");
    let db = database::connect(&settings).await.expect("database");
    database::migrate(&db).await.expect("migrate");
    clean(&db).await;
    Api {
        app: router(db.clone(), Secret::new(KEY.to_string())),
        db,
        _turn: turn,
    }
}

impl Api {
    pub async fn call(&self, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
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

    /// A call whose answer is not JSON (a file): the status, the headers and the raw bytes.
    pub async fn raw(&self, uri: &str) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
        let request = Request::builder()
            .method("GET")
            .uri(uri)
            .header(header::AUTHORIZATION, format!("Bearer {KEY}"))
            .body(Body::empty())
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        let (parts, body) = response.into_parts();
        let bytes = to_bytes(body, 1 << 22).await.unwrap();
        (parts.status, parts.headers, bytes.to_vec())
    }

    /// Creates a branch and returns its id.
    pub async fn create_branch(&self, number: &str) -> String {
        let (status, body) = self
            .call("POST", "/branches", Some(branch_input(number)))
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body["id"].as_str().unwrap().to_string()
    }

    pub async fn finish(self) {
        clean(&self.db).await;
    }
}

pub fn branch_input(number: &str) -> Value {
    json!({
        "number": number, "name": "Test Branch",
        "address_line1": "1 Industrial Rd", "address_line2": "Unit 4", "city": "Regina",
        "province": "SK", "postal_code": "s4p3y2", "phone": "306-555-0100",
        "latitude": 50.4452, "longitude": -104.6189
    })
}

/// A place the log lines go instead of the screen, so a test can read them.
#[derive(Clone, Default)]
pub struct LogText(pub Arc<Mutex<Vec<u8>>>);

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

pub fn customer_input(name: &str) -> Value {
    json!({
        "name": name, "company": "Prairie Farms Ltd.", "designation": "Service Manager",
        "phone": "306-555-0142", "alternate_phone": "(306) 555-0143", "email": "Jane@Example.COM",
        "address_line1": "123 Main St", "address_line2": "Suite 2", "city": "Regina",
        "province": "SK", "postal_code": "s4p3y2"
    })
}

impl Api {
    pub async fn create_customer(&self, name: &str) -> String {
        let (status, body) = self
            .call("POST", "/customers", Some(customer_input(name)))
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body["id"].as_str().unwrap().to_string()
    }
}
