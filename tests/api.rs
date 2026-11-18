// tests/api.rs: the web API's key check, input checks and settings (public API only)
//
// These calls go straight into the router (no network, no database): every one of them is
// answered before the database would be touched.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use quoto::api::{ApiSettings, router};
use quoto::*;
use sea_orm::DatabaseConnection;
use serde_json::{Value, json};
use tower::ServiceExt;

const KEY: &str = "test-key-test-key-test-key-test-key-1234";

fn app() -> Router {
    router(DatabaseConnection::default(), Secret::new(KEY.to_string()))
}

/// Makes one call and returns the status and the JSON that came back.
async fn call(
    method: &str,
    uri: &str,
    key: Option<&str>,
    body: Option<&str>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(key) = key {
        request = request.header(header::AUTHORIZATION, format!("Bearer {key}"));
    }
    if body.is_some() {
        request = request.header(header::CONTENT_TYPE, "application/json");
    }
    let request = request
        .body(Body::from(body.unwrap_or("").to_string()))
        .unwrap();
    let response = app().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1 << 20).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn branch_json() -> Value {
    json!({
        "number": "T-01", "name": "Test Branch",
        "address_line1": "1 Industrial Rd", "city": "Regina", "province": "SK",
        "postal_code": "S4P 3Y2", "phone": "306-555-0100"
    })
}

async fn post(body: Value) -> (StatusCode, Value) {
    call("POST", "/branches", Some(KEY), Some(&body.to_string())).await
}

// ===== the key =====

#[tokio::test]
async fn a_call_without_a_key_is_refused() {
    let (status, body) = call("GET", "/branches", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "UNAUTHORIZED");
}

#[tokio::test]
async fn a_wrong_key_is_refused_even_when_it_is_almost_right() {
    let almost = format!("{}5", &KEY[..KEY.len() - 1]);
    for wrong in ["wrong", "", &almost, &KEY[..KEY.len() - 1]] {
        let (status, _) = call("GET", "/branches", Some(wrong), None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{wrong:?}");
    }
}

#[tokio::test]
async fn the_key_must_come_as_a_bearer_token() {
    for header_value in [
        KEY.to_string(),
        format!("Basic {KEY}"),
        format!("bearer {KEY}"),
    ] {
        let request = Request::builder()
            .uri("/branches")
            .header(header::AUTHORIZATION, header_value)
            .body(Body::empty())
            .unwrap();
        let response = app().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}

#[tokio::test]
async fn a_refusal_says_how_to_log_in_and_never_repeats_the_key() {
    let request = Request::builder()
        .uri("/branches")
        .header(
            header::AUTHORIZATION,
            "Bearer secret-guess-secret-guess-123456",
        )
        .body(Body::empty())
        .unwrap();
    let response = app().oneshot(request).await.unwrap();
    assert_eq!(response.headers()[header::WWW_AUTHENTICATE], "Bearer");
    let bytes = to_bytes(response.into_body(), 1 << 20).await.unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(!text.contains("secret-guess") && !text.contains(KEY));
}

#[tokio::test]
async fn routes_that_do_not_exist_also_need_the_key() {
    // So a stranger can't learn which routes exist.
    let (status, _) = call("GET", "/nothing-here", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ===== a branch is never deleted =====

#[tokio::test]
async fn there_is_no_delete() {
    let id = "00000000-0000-0000-0000-000000000000";
    for uri in ["/branches", &format!("/branches/{id}")] {
        let (status, _) = call("DELETE", uri, Some(KEY), None).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{uri}");
    }
}

// ===== wrong input is refused before the database =====

#[tokio::test]
async fn a_body_that_is_not_the_expected_json_is_refused() {
    for body in ["", "not json", "[]", "{}", r#"{"number":"T-01"}"#] {
        let (status, answer) = call("POST", "/branches", Some(KEY), Some(body)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body:?}");
        assert_eq!(answer["code"], "INVALID_REQUEST_BODY");
    }
}

#[tokio::test]
async fn a_caller_cannot_choose_the_status_or_the_id_of_a_new_branch() {
    for extra in ["status", "id"] {
        let mut body = branch_json();
        body[extra] = json!("inactive");
        let (status, answer) = post(body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{extra}");
        assert_eq!(answer["code"], "INVALID_REQUEST_BODY");
    }
}

#[tokio::test]
async fn a_body_that_is_too_big_is_refused() {
    let mut body = branch_json();
    body["name"] = json!("x".repeat(20_000));
    let (status, _) = post(body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn the_librarys_rules_answer_with_their_own_codes() {
    let cases = [
        (
            "phone",
            json!("12345"),
            PhoneNumber::parse("12345").unwrap_err().code(),
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
                field: "branch name",
            }
            .code(),
        ),
        (
            "number",
            json!(""),
            QuoteError::MissingField {
                field: "branch number",
            }
            .code(),
        ),
    ];
    for (field, value, code) in cases {
        let mut body = branch_json();
        body[field] = value;
        let (status, answer) = post(body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{field}");
        assert_eq!(answer["code"], code, "{field}");
    }
}

#[tokio::test]
async fn half_a_map_location_is_refused() {
    let mut body = branch_json();
    body["latitude"] = json!(50.4);
    let (status, answer) = post(body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(answer["code"], QuoteError::InvalidLocation.code());
}

#[tokio::test]
async fn a_status_that_is_not_active_or_inactive_is_refused() {
    let id = "00000000-0000-0000-0000-000000000000";
    let (status, answer) = call(
        "PATCH",
        &format!("/branches/{id}/status"),
        Some(KEY),
        Some(r#"{"status":"deleted"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(answer["code"], "INVALID_STATUS");

    let (status, _) = call("GET", "/branches?status=in_active", Some(KEY), None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn an_id_that_is_not_a_uuid_is_a_branch_nobody_has() {
    let (status, answer) = call("GET", "/branches/not-an-id", Some(KEY), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(answer["code"], "UNKNOWN_BRANCH");
}

// ===== settings =====

fn settings(pairs: &[(&str, &str)]) -> QuoteResult<ApiSettings> {
    ApiSettings::from_lookup(|name| {
        pairs
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| v.to_string())
    })
}

#[test]
fn the_api_needs_a_key_and_has_no_default_one() {
    assert_eq!(
        settings(&[]).unwrap_err(),
        QuoteError::MissingSetting { name: "API_KEY" }
    );
    assert_eq!(
        settings(&[("API_KEY", "   ")]).unwrap_err(),
        QuoteError::MissingSetting { name: "API_KEY" }
    );
}

#[test]
fn a_short_key_is_refused() {
    assert_eq!(
        settings(&[("API_KEY", "short-key")]).unwrap_err(),
        QuoteError::InvalidSetting { name: "API_KEY" }
    );
    assert!(settings(&[("API_KEY", KEY)]).is_ok());
}

#[test]
fn it_listens_on_this_computer_port_3000_unless_told_otherwise() {
    let default = settings(&[("API_KEY", KEY)]).unwrap();
    assert_eq!(default.address().to_string(), "127.0.0.1:3000");
    let other = settings(&[("API_KEY", KEY), ("API_PORT", "8080")]).unwrap();
    assert_eq!(other.address().to_string(), "127.0.0.1:8080");
}

#[test]
fn it_refuses_to_listen_on_the_network_or_on_nonsense() {
    for (name, value) in [
        ("API_HOST", "0.0.0.0"),
        ("API_HOST", "192.168.1.5"),
        ("API_HOST", "localhost"),
        ("API_PORT", "http"),
        ("API_PORT", "70000"),
    ] {
        assert_eq!(
            settings(&[("API_KEY", KEY), (name, value)]).unwrap_err(),
            QuoteError::InvalidSetting { name },
            "{name}={value}"
        );
    }
}

#[test]
fn the_key_is_never_shown_when_the_settings_are_printed() {
    let text = format!("{:?}", settings(&[("API_KEY", KEY)]).unwrap());
    assert!(!text.contains(KEY));
}

// ===== the documentation page (the only calls that don't need the key) =====

async fn open(uri: &str) -> (StatusCode, String) {
    let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let response = app().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 4 << 20).await.unwrap();
    (status, String::from_utf8_lossy(&bytes).to_string())
}

async fn spec() -> Value {
    let (status, text) = open("/openapi.json").await;
    assert_eq!(status, StatusCode::OK);
    serde_json::from_str(&text).unwrap()
}

#[tokio::test]
async fn the_docs_page_and_the_list_of_calls_open_without_a_key() {
    let (status, page) = open("/docs/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.to_lowercase().contains("swagger"));
    assert_eq!(open("/openapi.json").await.0, StatusCode::OK);
}

#[tokio::test]
async fn everything_else_still_needs_the_key_next_to_the_docs() {
    for uri in [
        "/branches",
        "/nothing-here",
        "/docs-secret",
        "/openapi.jsonx",
    ] {
        assert_eq!(open(uri).await.0, StatusCode::UNAUTHORIZED, "{uri}");
    }
}

#[tokio::test]
async fn the_list_describes_exactly_the_five_branch_calls_and_no_delete() {
    let spec = spec().await;
    let mut calls: Vec<String> = spec["paths"]
        .as_object()
        .unwrap()
        .iter()
        .flat_map(|(path, methods)| {
            methods
                .as_object()
                .unwrap()
                .keys()
                .map(move |method| format!("{} {path}", method.to_uppercase()))
        })
        .collect();
    calls.sort();
    assert_eq!(
        calls,
        [
            "GET /branches",
            "GET /branches/{id}",
            "PATCH /branches/{id}/status",
            "POST /branches",
            "PUT /branches/{id}",
        ]
    );
}

#[tokio::test]
async fn every_described_call_really_exists_and_wants_the_key() {
    let spec = spec().await;
    let nil = "00000000-0000-0000-0000-000000000000";
    for (path, methods) in spec["paths"].as_object().unwrap() {
        for (method, details) in methods.as_object().unwrap() {
            let uri = path.replace("{id}", nil);
            // Without the key: refused, so the call is real and locked.
            let (status, _) = call(&method.to_uppercase(), &uri, None, None).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {path}");
            // The page says the key is needed (the lock icon).
            assert!(
                details.get("security").is_some() || spec["security"].is_array(),
                "{method} {path}"
            );
        }
    }
}

#[tokio::test]
async fn the_page_knows_the_key_goes_in_a_bearer_header() {
    let spec = spec().await;
    let scheme = &spec["components"]["securitySchemes"]["api_key"];
    assert_eq!(scheme["type"], "http");
    assert_eq!(scheme["scheme"], "bearer");
    assert_eq!(spec["security"][0], json!({"api_key": []}));
}

#[tokio::test]
async fn a_failed_call_is_described_as_code_and_message() {
    let spec = spec().await;
    let error = &spec["components"]["schemas"]["ErrorBody"]["properties"];
    assert!(error.get("code").is_some() && error.get("message").is_some());
}
