// tests/database_api/customers.rs: the customer calls with a real database behind them

use super::support::{customer_input, setup};
use axum::http::StatusCode;
use serde_json::{Value, json};

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
