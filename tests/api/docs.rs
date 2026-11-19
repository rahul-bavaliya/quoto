// tests/api/docs.rs: the documentation page (the only calls that don't need the key) and the list of calls

use super::support::{app, call};
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use tower::ServiceExt;

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
async fn the_list_describes_exactly_the_calls_and_only_equipment_has_a_delete() {
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
            "DELETE /equipment/{id}",
            "GET /branches",
            "GET /branches/{id}",
            "GET /customers",
            "GET /customers/{id}",
            "GET /equipment",
            "GET /equipment/{id}",
            "GET /quotes",
            "GET /quotes/{id}",
            "GET /quotes/{id}/pdf",
            "PATCH /branches/{id}",
            "PATCH /branches/{id}/status",
            "PATCH /customers/{id}",
            "PATCH /customers/{id}/status",
            "PATCH /equipment/{id}",
            "PATCH /equipment/{id}/owner",
            "PATCH /equipment/{id}/status",
            "PATCH /quotes/{id}",
            "PATCH /quotes/{id}/customer",
            "PATCH /quotes/{id}/equipment",
            "POST /branches",
            "POST /customers",
            "POST /equipment",
            "POST /quotes",
            "POST /quotes/{id}/accept",
            "POST /quotes/{id}/lines",
            "POST /quotes/{id}/reject",
            "POST /quotes/{id}/reopen",
            "PUT /branches/{id}",
            "PUT /customers/{id}",
            "PUT /equipment/{id}",
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

#[tokio::test]
async fn every_call_has_its_own_operation_id() {
    // The docs page keeps each call's answer under its operation id. Two calls sharing one
    // (both lists were called "list") made the customers panel show the branches answer.
    let spec = spec().await;
    let mut ids: Vec<String> = Vec::new();
    for (path, methods) in spec["paths"].as_object().unwrap() {
        for (method, details) in methods.as_object().unwrap() {
            let id = details["operationId"]
                .as_str()
                .unwrap_or_else(|| panic!("{method} {path} has no operation id"));
            ids.push(id.to_string());
        }
    }
    let mut unique = ids.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), ids.len(), "shared operation ids in {ids:?}");
}

/// The schema a call's request body points to (Day 56: two bodies were once both called
/// `CustomerInput`, so the page showed the quote's `customer_id` form for "create a customer").
fn body_properties<'a>(spec: &'a Value, path: &str, method: &str) -> Vec<&'a str> {
    let pointer =
        spec["paths"][path][method]["requestBody"]["content"]["application/json"]["schema"]["$ref"]
            .as_str()
            .unwrap_or_else(|| panic!("{method} {path} has no body schema"));
    let name = pointer.rsplit('/').next().unwrap();
    spec["components"]["schemas"][name]["properties"]
        .as_object()
        .unwrap_or_else(|| panic!("schema {name} has no properties"))
        .keys()
        .map(String::as_str)
        .collect()
}

#[tokio::test]
async fn each_call_shows_the_body_it_really_takes_and_no_two_bodies_share_a_name() {
    let spec = spec().await;
    let create_customer = body_properties(&spec, "/customers", "post");
    assert!(create_customer.contains(&"name") && !create_customer.contains(&"customer_id"));
    let create_machine = body_properties(&spec, "/equipment", "post");
    assert!(create_machine.contains(&"make") && !create_machine.contains(&"equipment_id"));
    assert_eq!(
        body_properties(&spec, "/quotes/{id}/customer", "patch"),
        ["customer_id"]
    );
    assert_eq!(
        body_properties(&spec, "/quotes/{id}/equipment", "patch"),
        ["equipment_id"]
    );

    // Every call with a body points at a schema of its own: two calls may share one only when
    // they really take the same body (e.g. the status call of every record).
    let mut seen: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for (path, item) in spec["paths"].as_object().unwrap() {
        for (method, call) in item.as_object().unwrap() {
            if let Some(pointer) =
                call["requestBody"]["content"]["application/json"]["schema"]["$ref"].as_str()
            {
                seen.entry(pointer.to_string())
                    .or_default()
                    .push(format!("{method} {path}"));
            }
        }
    }
    for (schema, calls) in &seen {
        if schema.ends_with("StatusInput") {
            continue;
        }
        // create + replace of one record legitimately share one input schema
        let records: std::collections::BTreeSet<&str> = calls
            .iter()
            .map(|call| call.split('/').nth(1).unwrap_or(""))
            .collect();
        assert_eq!(
            records.len(),
            1,
            "{schema} is used by different records: {calls:?}"
        );
    }
}
