// tests/database_api/equipment.rs: the equipment calls with a real database behind them
//
// Serial numbers here start with "API-T-" so the tests can remove their own rows.

use super::support::{Api, customer_input, setup};
use axum::http::StatusCode;
use serde_json::{Value, json};

fn input(serial: &str) -> Value {
    json!({
        "make": "John Deere", "model": "8R 310", "model_year": 2021,
        "serial_number": serial, "plate": "TR-1234",
        "meter_reading": 1250.5, "meter_type": "hr", "equipment_type": "tractor"
    })
}

impl Api {
    async fn machine(&self, serial: &str) -> String {
        let (status, body) = self.call("POST", "/equipment", Some(input(serial))).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body["id"].as_str().unwrap().to_string()
    }
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_machine_can_be_created_with_no_customer() {
    let api = setup().await;
    let (status, body) = api
        .call("POST", "/equipment", Some(input("api-t-one")))
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["customer_id"], Value::Null);
    assert_eq!(body["status"], "active");
    assert_eq!(body["serial_number"], "API-T-ONE");
    assert_eq!(body["meter_type"], "hr");
    assert_eq!(body["equipment_type"], "tractor");
    assert_eq!(body["source"], "entered by hand");
    assert_eq!(body["id"].as_str().unwrap().len(), 36);
    let (status, again) = api
        .call(
            "GET",
            &format!("/equipment/{}", body["id"].as_str().unwrap()),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(again, body);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_machine_can_be_created_for_a_customer_and_one_customer_has_many() {
    let api = setup().await;
    let owner = api.create_customer("API-T-Owner").await;
    let mut first = input("API-T-A");
    first["customer_id"] = json!(owner);
    let mut second = input("API-T-B");
    second["customer_id"] = json!(owner);
    for body in [first, second] {
        let (status, saved) = api.call("POST", "/equipment", Some(body)).await;
        assert_eq!(status, StatusCode::CREATED, "{saved}");
        assert_eq!(saved["customer_id"], owner);
    }
    api.machine("API-T-NOBODY").await;
    let (_, mine) = api
        .call("GET", &format!("/equipment?customer_id={owner}"), None)
        .await;
    assert_eq!(mine.as_array().unwrap().len(), 2);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn an_owner_that_does_not_exist_or_is_inactive_is_refused_and_nothing_is_saved() {
    let api = setup().await;
    let mut ghost = input("API-T-GHOST");
    ghost["customer_id"] = json!("00000000-0000-0000-0000-000000000000");
    let (status, body) = api.call("POST", "/equipment", Some(ghost)).await;
    assert_eq!(
        (status, body["code"].as_str()),
        (StatusCode::NOT_FOUND, Some("UNKNOWN_CUSTOMER"))
    );

    let closed = api.create_customer("API-T-Closed").await;
    api.call(
        "PATCH",
        &format!("/customers/{closed}/status"),
        Some(json!({"status": "inactive"})),
    )
    .await;
    let mut shut = input("API-T-SHUT");
    shut["customer_id"] = json!(closed);
    let (status, body) = api.call("POST", "/equipment", Some(shut)).await;
    assert_eq!(
        (status, body["code"].as_str()),
        (StatusCode::CONFLICT, Some("CUSTOMER_INACTIVE"))
    );

    let (_, found) = api
        .call("GET", "/equipment?serial_number=api-t-ghost", None)
        .await;
    assert!(found.as_array().unwrap().is_empty());
    let (_, found) = api
        .call("GET", "/equipment?serial_number=api-t-shut", None)
        .await;
    assert!(found.as_array().unwrap().is_empty());
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_same_serial_number_is_a_409_that_says_to_change_the_owner() {
    let api = setup().await;
    api.machine("API-T-DUP").await;
    for again in ["API-T-DUP", "api-t-dup"] {
        let (status, body) = api.call("POST", "/equipment", Some(input(again))).await;
        assert_eq!(status, StatusCode::CONFLICT, "{again}");
        assert_eq!(body["code"], "DUPLICATE_SERIAL_NUMBER");
        assert!(
            body["message"].as_str().unwrap().contains("owner"),
            "{body}"
        );
        assert!(
            !body.to_string().contains("API-T-DUP"),
            "the message repeats the serial: {body}"
        );
    }
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_sale_moves_the_same_machine_to_another_customer() {
    let api = setup().await;
    let first = api.create_customer("API-T-Seller").await;
    let second = api.create_customer("API-T-Buyer").await;
    let id = api.machine("API-T-SOLD").await;
    let uri = format!("/equipment/{id}/owner");

    let (status, body) = api
        .call("PATCH", &uri, Some(json!({"customer_id": first})))
        .await;
    assert_eq!(
        (status, &body["customer_id"]),
        (StatusCode::OK, &json!(first))
    );
    let (_, body) = api
        .call("PATCH", &uri, Some(json!({"customer_id": second})))
        .await;
    assert_eq!(body["customer_id"], second);
    assert_eq!(body["id"], id); // the same record
    let (_, mine) = api
        .call("GET", &format!("/equipment?customer_id={first}"), None)
        .await;
    assert!(mine.as_array().unwrap().is_empty());
    let (_, body) = api
        .call("PATCH", &uri, Some(json!({"customer_id": null})))
        .await;
    assert_eq!(body["customer_id"], Value::Null);
    let (_, free) = api
        .call(
            "GET",
            "/equipment?unowned=true&serial_number=api-t-sold",
            None,
        )
        .await;
    assert_eq!(free.as_array().unwrap().len(), 1);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_owner_call_refuses_a_missing_or_inactive_customer_and_keeps_the_owner() {
    let api = setup().await;
    let owner = api.create_customer("API-T-Keeper").await;
    let id = api.machine("API-T-KEEP").await;
    let uri = format!("/equipment/{id}/owner");
    api.call("PATCH", &uri, Some(json!({"customer_id": owner})))
        .await;

    let (status, body) = api
        .call(
            "PATCH",
            &uri,
            Some(json!({"customer_id": "00000000-0000-0000-0000-000000000000"})),
        )
        .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (StatusCode::NOT_FOUND, Some("UNKNOWN_CUSTOMER"))
    );
    let closed = api.create_customer("API-T-Closed").await;
    api.call(
        "PATCH",
        &format!("/customers/{closed}/status"),
        Some(json!({"status": "inactive"})),
    )
    .await;
    let (status, body) = api
        .call("PATCH", &uri, Some(json!({"customer_id": closed})))
        .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (StatusCode::CONFLICT, Some("CUSTOMER_INACTIVE"))
    );
    let (_, now) = api.call("GET", &format!("/equipment/{id}"), None).await;
    assert_eq!(now["customer_id"], owner);

    let (status, body) = api
        .call(
            "PATCH",
            "/equipment/00000000-0000-0000-0000-000000000000/owner",
            Some(json!({"customer_id": owner})),
        )
        .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (StatusCode::NOT_FOUND, Some("UNKNOWN_EQUIPMENT"))
    );
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn delete_archives_the_machine_and_the_status_call_restores_it() {
    let api = setup().await;
    let id = api.machine("API-T-ARCH").await;
    let (status, body) = api.call("DELETE", &format!("/equipment/{id}"), None).await;
    assert_eq!(
        (status, &body["status"]),
        (StatusCode::OK, &json!("inactive"))
    );
    assert_eq!(body["make"], "John Deere"); // nothing else changed

    // still readable, hidden from the active list, shown in the archived one
    let (status, _) = api.call("GET", &format!("/equipment/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    let (_, active) = api
        .call(
            "GET",
            "/equipment?status=active&serial_number=api-t-arch",
            None,
        )
        .await;
    assert!(active.as_array().unwrap().is_empty());
    let (_, archived) = api
        .call(
            "GET",
            "/equipment?status=inactive&serial_number=api-t-arch",
            None,
        )
        .await;
    assert_eq!(archived.as_array().unwrap().len(), 1);
    // deleting again is fine (still archived); the serial stays taken
    let (status, _) = api.call("DELETE", &format!("/equipment/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = api
        .call("POST", "/equipment", Some(input("API-T-ARCH")))
        .await;
    assert_eq!(status, StatusCode::CONFLICT);

    let (_, body) = api
        .call(
            "PATCH",
            &format!("/equipment/{id}/status"),
            Some(json!({"status": "active"})),
        )
        .await;
    assert_eq!(body["status"], "active");
    let (status, body) = api
        .call(
            "DELETE",
            "/equipment/00000000-0000-0000-0000-000000000000",
            None,
        )
        .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (StatusCode::NOT_FOUND, Some("UNKNOWN_EQUIPMENT"))
    );
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn put_replaces_the_details_and_keeps_owner_and_status() {
    let api = setup().await;
    let owner = api.create_customer("API-T-PutOwner").await;
    let id = api.machine("API-T-PUT").await;
    api.call(
        "PATCH",
        &format!("/equipment/{id}/owner"),
        Some(json!({"customer_id": owner})),
    )
    .await;
    api.call("DELETE", &format!("/equipment/{id}"), None).await;

    let replacement = json!({
        "make": "Ford", "model": "F-150", "serial_number": "api-t-put-2",
        "meter_type": "km", "equipment_type": "truck"
    });
    let (status, body) = api
        .call("PUT", &format!("/equipment/{id}"), Some(replacement))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["make"], "Ford");
    assert_eq!(body["serial_number"], "API-T-PUT-2");
    for cleared in ["model_year", "plate", "meter_reading"] {
        assert_eq!(body[cleared], Value::Null, "{cleared}");
    }
    assert_eq!(body["customer_id"], owner);
    assert_eq!(body["status"], "inactive");

    let (status, body) = api
        .call(
            "PUT",
            &format!("/equipment/{id}"),
            Some(json!({"make": "Ford"})),
        )
        .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (StatusCode::BAD_REQUEST, Some("INVALID_REQUEST_BODY"))
    );
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn patch_changes_only_what_is_sent_and_null_clears_an_optional_detail() {
    let api = setup().await;
    let id = api.machine("API-T-PATCH").await;
    let uri = format!("/equipment/{id}");
    let (status, body) = api
        .call(
            "PATCH",
            &uri,
            Some(json!({"meter_reading": 1300.0, "plate": null})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["meter_reading"], 1300.0);
    assert_eq!(body["plate"], Value::Null);
    assert_eq!(body["make"], "John Deere"); // untouched
    assert_eq!(body["serial_number"], "API-T-PATCH");
    assert_eq!(body["model_year"], 2021);

    // a required detail can't be cleared, and a bad value changes nothing
    for bad in [
        json!({"make": null}),
        json!({"meter_type": "parsecs"}),
        json!({"meter_reading": -1.0}),
        json!({"serial_number": null, "plate": null}),
    ] {
        let (status, _) = api.call("PATCH", &uri, Some(bad.clone())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad}");
    }
    let (_, now) = api.call("GET", &uri, None).await;
    assert_eq!(now["make"], "John Deere");
    assert_eq!(now["serial_number"], "API-T-PATCH");
    assert_eq!(now["meter_type"], "hr");

    // taking another machine's serial number is a 409
    api.machine("API-T-OTHER").await;
    let (status, body) = api
        .call("PATCH", &uri, Some(json!({"serial_number": "api-t-other"})))
        .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (StatusCode::CONFLICT, Some("DUPLICATE_SERIAL_NUMBER"))
    );
    // keeping its own is fine
    let (status, _) = api
        .call("PATCH", &uri, Some(json!({"serial_number": "api-t-patch"})))
        .await;
    assert_eq!(status, StatusCode::OK);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_list_pages_filters_and_defaults_work() {
    let api = setup().await;
    for n in 1..=3 {
        api.machine(&format!("API-T-LIST-{n}")).await;
    }
    let (status, page) = api
        .call("GET", "/equipment?serial_number=api-t-list-2", None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page.as_array().unwrap().len(), 1);
    let (_, all) = api.call("GET", "/equipment?limit=200", None).await;
    let (_, second) = api.call("GET", "/equipment?limit=1&offset=1", None).await;
    assert_eq!(second.as_array().unwrap().len(), 1);
    assert_eq!(second[0], all[1]);
    let (_, clamped) = api.call("GET", "/equipment?limit=0", None).await;
    assert_eq!(clamped.as_array().unwrap().len(), 1); // 0 is raised to 1
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn an_unknown_machine_is_a_404_everywhere() {
    let api = setup().await;
    let nil = "00000000-0000-0000-0000-000000000000";
    for (method, uri, body) in [
        ("GET", format!("/equipment/{nil}"), None),
        (
            "PUT",
            format!("/equipment/{nil}"),
            Some(input("API-T-NONE")),
        ),
        (
            "PATCH",
            format!("/equipment/{nil}"),
            Some(json!({"plate": "X"})),
        ),
        (
            "PATCH",
            format!("/equipment/{nil}/status"),
            Some(json!({"status": "inactive"})),
        ),
    ] {
        let (status, body) = api.call(method, &uri, body).await;
        assert_eq!(
            (status, body["code"].as_str()),
            (StatusCode::NOT_FOUND, Some("UNKNOWN_EQUIPMENT")),
            "{method} {uri}"
        );
    }
    let _ = customer_input; // (the customer helpers live in support)
    api.finish().await;
}
