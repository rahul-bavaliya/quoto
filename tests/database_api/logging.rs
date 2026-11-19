// tests/database_api/logging.rs: what the log says about real calls (rules: claude/logging-rules.md)

use super::support::{KEY, LogText, branch_input, clean, customer_input, setup};
use quoto::*;
use serde_json::json;

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

    let mut body = branch_input("API-T-LOG");
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

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn patch_calls_log_nothing_about_what_was_sent() {
    let api = setup().await;
    let branch = api.create_branch("API-T-PL").await;
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

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn equipment_calls_log_nothing_about_the_machine_or_its_owner() {
    let api = setup().await;
    let log = LogText::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(log.clone())
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let owner = api.create_customer("API-T-Quiet Owner").await;
    let body = json!({
        "make": "Zebulon Motors", "model": "Quietmodel 9", "model_year": 2019,
        "serial_number": "API-T-SECRETSERIAL1", "plate": "SECRETPLATE9",
        "meter_reading": 4321.5, "meter_type": "hr", "equipment_type": "tractor",
        "customer_id": owner
    });
    let (_, created) = api.call("POST", "/equipment", Some(body)).await;
    let id = created["id"].as_str().unwrap().to_string();
    api.call("GET", &format!("/equipment/{id}"), None).await;
    api.call(
        "GET",
        "/equipment?serial_number=API-T-SECRETSERIAL1&limit=5",
        None,
    )
    .await;
    api.call(
        "PATCH",
        &format!("/equipment/{id}"),
        Some(json!({"plate": "SECRETPLATE8"})),
    )
    .await;
    api.call(
        "PATCH",
        &format!("/equipment/{id}/owner"),
        Some(json!({"customer_id": null})),
    )
    .await;
    // a refused duplicate must not echo the serial either
    api.call("POST", "/equipment", Some(json!({
        "make": "Zebulon Motors", "model": "Quietmodel 9", "serial_number": "API-T-SECRETSERIAL1",
        "meter_type": "hr", "equipment_type": "tractor"
    }))).await;
    api.call("DELETE", &format!("/equipment/{id}"), None).await;

    let text = String::from_utf8_lossy(&log.0.lock().unwrap().clone()).to_string();
    for secret in [
        id.as_str(),
        owner.as_str(),
        "Zebulon",
        "Quietmodel",
        "SECRETSERIAL",
        "SECRETPLATE",
        "4321.5",
        "Quiet Owner",
        KEY,
    ] {
        assert!(
            !text.contains(secret),
            "{secret} leaked into the log:\n{text}"
        );
    }
    let calls: Vec<&str> = text.lines().filter(|l| l.contains("api_request")).collect();
    assert_eq!(calls.len(), 8, "{text}"); // the customer create counts too
    assert!(
        calls[2].contains("route=\"/equipment/{id}\""),
        "{}",
        calls[2]
    );
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn quote_calls_log_nothing_about_the_quote() {
    let api = setup().await;
    let branch = api.create_branch("API-T-QLOG").await;
    let customer = api.create_customer("API-T-Zed Quote Person").await;
    let (_, machine) = api
        .call(
            "POST",
            "/equipment",
            Some(json!({
                "make": "Zed Make", "model": "Zed Model", "serial_number": "API-T-QLOG-SERIAL",
                "plate": "ZED-999", "meter_type": "hr", "meter_reading": 4321.5,
                "equipment_type": "tractor", "customer_id": customer
            })),
        )
        .await;
    let machine = machine["id"].as_str().unwrap().to_string();

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
            "/quotes",
            Some(json!({
                "branch_id": branch, "customer_id": customer, "equipment_id": machine,
                "items": [{"description": "Zed Secret Labour", "quantity": 1, "unit_price_cents": 98765, "kind": "labour"}]
            })),
        )
        .await;
    let id = created["id"].as_str().unwrap().to_string();
    api.call("GET", &format!("/quotes/{id}"), None).await;
    api.call("GET", &format!("/quotes?customer_id={customer}"), None)
        .await;
    api.call(
        "POST",
        &format!("/quotes/{id}/lines"),
        Some(json!({"description": "Zed Secret Part", "quantity": 1, "unit_price_cents": 5551, "kind": "part"})),
    )
    .await;
    api.call(
        "POST",
        &format!("/quotes/{id}/reject"),
        Some(json!({"reason": "Zed customer said the secret reason"})),
    )
    .await;
    api.call("POST", &format!("/quotes/{id}/reopen"), None)
        .await;
    api.call("POST", &format!("/quotes/{id}/accept"), None)
        .await;
    // refused calls too
    api.call(
        "POST",
        &format!("/quotes/{id}/reject"),
        Some(json!({"reason": "Zed late secret reason"})),
    )
    .await;
    api.call(
        "PATCH",
        &format!("/quotes/{id}"),
        Some(json!({"promise_date": "2031-02-03"})),
    )
    .await;

    // the PDF holds every detail of the quote; the log must still hold none of it
    let (_, _, pdf) = api.raw(&format!("/quotes/{id}/pdf")).await;
    assert!(String::from_utf8_lossy(&pdf).contains("Zed Quote Person"));
    let text = String::from_utf8_lossy(&log.0.lock().unwrap().clone()).to_string();
    for secret in [
        id.as_str(),
        customer.as_str(),
        machine.as_str(),
        branch.as_str(),
        "Zed Quote Person",
        "Zed Make",
        "Zed Model",
        "API-T-QLOG",
        "ZED-999",
        "4321.5",
        "Zed Secret Labour",
        "Zed Secret Part",
        "secret reason",
        "late secret",
        "2031-02-03",
        "Prairie Farms",
        "123 Main St",
        KEY,
    ] {
        assert!(
            !text.contains(secret),
            "{secret} leaked into the log:\n{text}"
        );
    }
    let calls: Vec<&str> = text.lines().filter(|l| l.contains("api_request")).collect();
    assert_eq!(calls.len(), 10, "{text}");
    assert!(calls[0].contains("route=\"/quotes\"") && calls[0].contains("status=201"));
    assert!(calls[1].contains("route=\"/quotes/{id}\""));
    assert!(calls[4].contains("route=\"/quotes/{id}/reject\""));
    assert!(calls[7].contains("status=409") && calls[7].contains("code=\"ALREADY_DECIDED\""));
    assert!(calls[9].contains("route=\"/quotes/{id}/pdf\"") && calls[9].contains("status=200"));
    api.finish().await;
}
