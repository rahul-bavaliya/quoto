// tests/api_logging.rs: what the web API writes to the log (the rules: claude/logging-rules.md)
//
// Every call must leave ONE line with safe facts only. These tests send calls full of
// recognisable "secret" values and fail if any of them shows up in the log, at any level.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, Response, StatusCode, header};
use quoto::api::router;
use quoto::*;
use sea_orm::DatabaseConnection;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use tracing_subscriber::fmt::MakeWriter;

const KEY: &str = "test-key-test-key-test-key-test-key-1234";

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

impl LogText {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).to_string()
    }

    /// The lines about calls (one per call).
    fn call_lines(&self) -> Vec<String> {
        self.text()
            .lines()
            .filter(|line| line.contains("api_request"))
            .map(str::to_string)
            .collect()
    }
}

/// Starts capturing every log line (all levels, all crates) until the guard is dropped.
fn capture() -> (LogText, tracing::subscriber::DefaultGuard) {
    let log = LogText::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(log.clone())
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .finish();
    (log, tracing::subscriber::set_default(subscriber))
}

fn app() -> Router {
    router(DatabaseConnection::default(), Secret::new(KEY.to_string()))
}

async fn call(
    method: &str,
    uri: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> (Response<Body>, String) {
    let mut request = Request::builder().method(method).uri(uri);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = app()
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let (parts, body) = response.into_parts();
    let bytes = to_bytes(body, 4 << 20).await.unwrap();
    (
        Response::from_parts(parts, Body::empty()),
        String::from_utf8_lossy(&bytes).to_string(),
    )
}

fn with_key() -> [(&'static str, &'static str); 2] {
    [
        (
            "authorization",
            Box::leak(format!("Bearer {KEY}").into_boxed_str()),
        ),
        ("content-type", "application/json"),
    ]
}

// ===== one safe line per call =====

#[tokio::test]
async fn every_call_leaves_exactly_one_line() {
    let (log, _guard) = capture();
    call("GET", "/branches", &[], "").await;
    call("GET", "/branches?status=bad", &with_key(), "").await;
    call("POST", "/branches", &with_key(), "not json").await;
    call("GET", "/openapi.json", &[], "").await;
    assert_eq!(log.call_lines().len(), 4, "{}", log.text());
}

#[tokio::test]
async fn the_line_says_who_what_and_how_it_went_and_nothing_more() {
    let (log, _guard) = capture();
    call("POST", "/branches", &with_key(), "not json").await;
    let lines = log.call_lines();
    let line = &lines[0];
    for expected in [
        "WARN",
        "request_id=",
        "method=POST",
        "route=\"/branches\"",
        "status=400",
        "elapsed_ms=",
        "code=\"INVALID_REQUEST_BODY\"",
    ] {
        assert!(line.contains(expected), "{expected} missing in: {line}");
    }
}

#[tokio::test]
async fn a_good_call_is_info_with_no_error_code() {
    let (log, _guard) = capture();
    call("GET", "/openapi.json", &[], "").await;
    let line = log.call_lines().remove(0);
    assert!(
        line.contains("INFO") && line.contains("status=200"),
        "{line}"
    );
    assert!(line.contains("code=\"none\""), "{line}");
}

#[tokio::test]
async fn the_level_follows_the_status() {
    let (log, _guard) = capture();
    call("GET", "/branches", &[], "").await; // 401: the caller's problem
    call("GET", "/branches/not-an-id", &with_key(), "").await; // 404
    let lines = log.call_lines();
    assert!(lines.iter().all(|line| line.contains("WARN")), "{lines:?}");
    // (500 and above, our own failure, is `error`: see the database test of the 503 below.)
}

#[tokio::test]
async fn the_route_is_the_pattern_never_the_real_path() {
    let (log, _guard) = capture();
    call("GET", "/branches/real-looking-id-a8ac03a9", &with_key(), "").await;
    let line = log.call_lines().remove(0);
    assert!(line.contains("route=\"/branches/{id}\""), "{line}");
    assert!(!line.contains("a8ac03a9"), "{line}");
}

#[tokio::test]
async fn a_call_to_nowhere_is_logged_as_unmatched() {
    let (log, _guard) = capture();
    call("GET", "/stranger-probe-path", &[], "").await;
    let line = log.call_lines().remove(0);
    assert!(line.contains("route=\"unmatched\""), "{line}");
    assert!(line.contains("status=401"), "{line}");
}

// ===== nothing secret, ever =====

#[tokio::test]
async fn no_secret_value_reaches_the_log_at_any_level() {
    let (log, _guard) = capture();
    let secrets = [
        "wrong-key-should-never-be-logged-999",
        "Zed Quiet Branch",
        "306-555-0177",
        "S4P3Y2-secret-postal",
        "secret-query-value",
        "secret-path-segment",
        "forged-request-id-123",
    ];
    // A wrong key, a body full of values, a secret in the query and in the path, a forged id.
    call(
        "GET",
        "/branches?status=secret-query-value",
        &[(
            "authorization",
            "Bearer wrong-key-should-never-be-logged-999",
        )],
        "",
    )
    .await;
    call(
        "POST",
        "/branches",
        &[
            ("authorization", &format!("Bearer {KEY}")),
            ("content-type", "application/json"),
            ("x-request-id", "forged-request-id-123"),
        ],
        r#"{"number":"T-01","name":"Zed Quiet Branch","address_line1":"1 Industrial Rd",
            "city":"Regina","province":"SK","postal_code":"S4P3Y2-secret-postal","phone":"306-555-0177"}"#,
    )
    .await;
    call(
        "GET",
        "/branches?status=secret-query-value",
        &with_key(),
        "",
    )
    .await;
    call("GET", "/branches/secret-path-segment", &with_key(), "").await;
    call("GET", "/secret-path-segment/probe", &[], "").await;

    let text = log.text();
    assert!(!text.is_empty());
    for secret in secrets.iter().chain(&[KEY]) {
        assert!(
            !text.contains(secret),
            "{secret} leaked into the log:\n{text}"
        );
    }
}

#[tokio::test]
async fn the_key_check_and_the_error_both_leave_one_line_not_two() {
    let (log, _guard) = capture();
    call("GET", "/branches", &[], "").await;
    // The whole log for one refused call is that one line (no second "key refused" line).
    assert_eq!(log.text().lines().count(), 1, "{}", log.text());
}

// ===== the call number =====

#[tokio::test]
async fn the_caller_gets_a_call_number_to_quote_and_it_is_ours_not_theirs() {
    let (log, _guard) = capture();
    let (first, _) = call(
        "GET",
        "/openapi.json",
        &[("x-request-id", "forged-request-id-123")],
        "",
    )
    .await;
    let (second, _) = call("GET", "/openapi.json", &[], "").await;
    let number = |response: &Response<Body>| -> u64 {
        response.headers()["x-request-id"]
            .to_str()
            .unwrap()
            .parse()
            .unwrap()
    };
    assert!(number(&second) > number(&first));
    assert!(!log.text().contains("forged-request-id-123"));
    assert!(
        log.text()
            .contains(&format!("request_id={}", number(&first)))
    );
}

#[tokio::test]
async fn a_refused_call_still_gets_a_call_number_and_the_challenge() {
    let (response, body) = call("GET", "/branches", &[], "").await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(response.headers()[header::WWW_AUTHENTICATE], "Bearer");
    assert!(response.headers().contains_key("x-request-id"));
    assert!(body.contains("UNAUTHORIZED"));
}

#[tokio::test]
async fn no_customer_detail_reaches_the_log_at_any_level() {
    let (log, _guard) = capture();
    let secrets = [
        "Zed Quiet Person",
        "Hidden Company Ltd",
        "Chief Secret Officer",
        "quiet.person@secret-mail.example",
        "secret-query-for-customers",
        "secret-customer-path",
    ];
    // Refused (bad phone) before the database is reached, with all the personal details in the body.
    call(
        "POST",
        "/customers",
        &with_key(),
        r#"{"name":"Zed Quiet Person","company":"Hidden Company Ltd","designation":"Chief Secret Officer",
            "phone":"999","email":"quiet.person@secret-mail.example","address_line1":"1 Hidden Lane",
            "city":"Regina","province":"SK","postal_code":"S4P3Y2"}"#,
    )
    .await;
    call(
        "GET",
        "/customers?status=secret-query-for-customers",
        &with_key(),
        "",
    )
    .await;
    call("GET", "/customers/secret-customer-path", &with_key(), "").await;

    let text = log.text();
    assert_eq!(log.call_lines().len(), 3, "{text}");
    for secret in secrets.iter().chain(&["1 Hidden Lane", KEY]) {
        assert!(
            !text.contains(secret),
            "{secret} leaked into the log:\n{text}"
        );
    }
    assert!(text.contains("route=\"/customers/{id}\""));
}
