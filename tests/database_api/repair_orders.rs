// tests/database_api/repair_orders.rs: the repair order calls with a real database behind them
//
// Branches, customers and machines here start with "API-T-" (a repair order is found by its
// branch), and the tests remove their own orders, then give back the numbers they used
// (see `support::clean`).

use super::support::{Api, KEY, LogText, setup};
use axum::http::StatusCode;
use serde_json::{Value, json};

fn line(kind: &str, quantity: f64, price: i64) -> Value {
    json!({"description": format!("API-T {kind}"), "quantity": quantity, "unit_price_cents": price, "kind": kind})
}

struct Set {
    branch: String,
    customer: String,
    machine: String,
}

impl Api {
    /// A branch, a customer, and a machine that customer owns (with a meter reading, or without).
    async fn set_with(&self, tag: &str, reading: Option<f64>) -> Set {
        let branch = self.create_branch(&format!("API-T-R{tag}")).await;
        let customer = self.create_customer(&format!("API-T-Repairer {tag}")).await;
        let (status, machine) = self
            .call(
                "POST",
                "/equipment",
                Some(json!({
                    "make": "John Deere", "model": "8R 310", "model_year": 2021,
                    "serial_number": format!("API-T-R{tag}"), "plate": "TR-1234",
                    "meter_reading": reading, "meter_type": "hr", "equipment_type": "tractor",
                    "customer_id": customer
                })),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{machine}");
        Set {
            branch,
            customer,
            machine: machine["id"].as_str().unwrap().to_string(),
        }
    }

    /// An accepted quote for the set (the standard way a repair order starts).
    async fn accepted(&self, set: &Set) -> String {
        let (status, quote) = self
            .call(
                "POST",
                "/quotes",
                Some(json!({
                    "branch_id": set.branch, "customer_id": set.customer,
                    "equipment_id": set.machine,
                    "items": [line("labour", 2.0, 12000), line("part", 1.0, 1500)]
                })),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{quote}");
        let id = quote["id"].as_str().unwrap().to_string();
        let (status, accepted) = self
            .call("POST", &format!("/quotes/{id}/accept"), None)
            .await;
        assert_eq!(status, StatusCode::OK, "{accepted}");
        id
    }

    async fn open_from(&self, quote: &str, body: Value) -> (StatusCode, Value) {
        self.call("POST", &format!("/quotes/{quote}/repair-order"), Some(body))
            .await
    }

    async fn order(&self, set: &Set, tag: &str) -> Value {
        let _ = tag;
        let quote = self.accepted(set).await;
        let (status, order) = self.open_from(&quote, json!({})).await;
        assert_eq!(status, StatusCode::CREATED, "{order}");
        order
    }
}

fn id(order: &Value) -> &str {
    order["id"].as_str().unwrap()
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn an_accepted_quote_opens_a_repair_order_that_carries_the_quotes_work_and_the_meter() {
    let api = setup().await;
    let set = api.set_with("1", Some(1250.5)).await;
    let quote = api.accepted(&set).await;
    let (status, headers, order) = api
        .call_with(
            "POST",
            &format!("/quotes/{quote}/repair-order"),
            Some(json!({})),
            &[],
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{order}");
    assert_eq!(headers["etag"], "\"0\"");
    let number = order["number"].as_str().unwrap();
    assert!(number.starts_with("RO-") && number.len() == 16, "{number}");
    assert_eq!(order["version"], 0);
    assert_eq!(order["status"], "open");
    assert_eq!(order["cancellation_reason"], Value::Null);
    assert_eq!(order["quote_id"], quote.as_str());
    assert!(order["quote_number"].as_str().unwrap().starts_with("Q-"));
    assert_eq!(order["meter"], json!({"reading": 1250.5, "type": "hr"}));
    assert_eq!(order["started_at"], Value::Null);
    assert_eq!(order["closed_at"], Value::Null);
    assert_eq!(order["branch"]["number"], "API-T-R1");
    assert_eq!(order["customer"]["id"], set.customer.as_str());
    assert_eq!(order["equipment"]["id"], set.machine.as_str());
    assert_eq!(order["items"].as_array().unwrap().len(), 2);
    assert_eq!(order["items"][0]["line_total_cents"], 24000);
    assert_eq!(order["totals"]["subtotal_cents"], 25500);
    // reading it again gives the same thing
    let (status, headers, again) = api
        .call_with("GET", &format!("/repair-orders/{}", id(&order)), None, &[])
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["etag"], "\"0\"");
    assert_eq!(again, order);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_meter_reading_comes_from_the_call_or_from_the_machine_or_it_is_refused() {
    let api = setup().await;
    let with = api.set_with("2", Some(100.0)).await;
    let without = api.set_with("3", None).await;
    let quote_with = api.accepted(&with).await;
    let quote_without = api.accepted(&without).await;

    // today's reading wins over the machine's
    let (status, order) = api
        .open_from(&quote_with, json!({"meter_reading": 120.5}))
        .await;
    assert_eq!(status, StatusCode::CREATED, "{order}");
    assert_eq!(order["meter"]["reading"], 120.5);

    // no reading anywhere: refused, and the refusal used no number
    for body in [json!({}), json!({"meter_reading": null})] {
        assert_eq!(
            api.failure(
                "POST",
                &format!("/quotes/{quote_without}/repair-order"),
                Some(body)
            )
            .await,
            (400, "METER_READING_REQUIRED".to_string())
        );
    }
    for bad in [-1.0, -0.001] {
        assert_eq!(
            api.failure(
                "POST",
                &format!("/quotes/{quote_without}/repair-order"),
                Some(json!({"meter_reading": bad}))
            )
            .await,
            (400, "INVALID_METER_READING".to_string())
        );
    }
    let (status, ok) = api
        .open_from(&quote_without, json!({"meter_reading": 0}))
        .await;
    assert_eq!(status, StatusCode::CREATED, "{ok}"); // zero is a real reading
    let (first, second) = (
        order["number"].as_str().unwrap(),
        ok["number"].as_str().unwrap(),
    );
    assert_eq!(first[..8], second[..8]); // the same year
    assert_eq!(
        second[8..].parse::<u32>().unwrap(),
        first[8..].parse::<u32>().unwrap() + 1,
        "the refused calls took no number"
    );
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn only_an_accepted_quote_that_exists_opens_a_repair_order_and_only_once() {
    let api = setup().await;
    let set = api.set_with("4", Some(10.0)).await;
    let (status, pending) = api
        .call(
            "POST",
            "/quotes",
            Some(json!({"branch_id": set.branch, "customer_id": set.customer,
                        "equipment_id": set.machine, "items": [line("part", 1.0, 100)]})),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    let pending = pending["id"].as_str().unwrap().to_string();
    assert_eq!(
        api.failure(
            "POST",
            &format!("/quotes/{pending}/repair-order"),
            Some(json!({}))
        )
        .await,
        (409, "QUOTE_NOT_ACCEPTED".to_string())
    );
    api.call(
        "POST",
        &format!("/quotes/{pending}/reject"),
        Some(json!({"reason": "no"})),
    )
    .await;
    assert_eq!(
        api.failure(
            "POST",
            &format!("/quotes/{pending}/repair-order"),
            Some(json!({}))
        )
        .await,
        (409, "QUOTE_NOT_ACCEPTED".to_string())
    );
    assert_eq!(
        api.failure(
            "POST",
            "/quotes/00000000-0000-0000-0000-000000000000/repair-order",
            Some(json!({"meter_reading": 1}))
        )
        .await,
        (404, "UNKNOWN_QUOTE".to_string())
    );
    assert_eq!(
        api.failure(
            "GET",
            "/repair-orders/00000000-0000-0000-0000-000000000000",
            None
        )
        .await,
        (404, "UNKNOWN_REPAIR_ORDER".to_string())
    );

    let accepted = api.accepted(&set).await;
    let (status, _) = api.open_from(&accepted, json!({})).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        api.failure(
            "POST",
            &format!("/quotes/{accepted}/repair-order"),
            Some(json!({}))
        )
        .await,
        (409, "REPAIR_ORDER_EXISTS".to_string())
    );
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_order_goes_open_to_in_progress_to_completed_and_every_step_adds_one_to_the_version() {
    let api = setup().await;
    let set = api.set_with("5", Some(10.0)).await;
    let order = api.order(&set, "5").await;
    let base = format!("/repair-orders/{}", id(&order));

    // out of turn: completing an order that has not started
    assert_eq!(
        api.failure("POST", &format!("{base}/complete"), None).await,
        (409, "REPAIR_ORDER_NOT_IN_PROGRESS".to_string())
    );
    let (status, headers, started) = api
        .call_with("POST", &format!("{base}/start"), None, &[])
        .await;
    assert_eq!(status, StatusCode::OK, "{started}");
    assert_eq!(headers["etag"], "\"1\"");
    assert_eq!(started["status"], "in_progress");
    assert!(started["started_at"].is_string());
    assert_eq!(started["closed_at"], Value::Null);
    assert_eq!(
        api.failure("POST", &format!("{base}/start"), None).await,
        (409, "REPAIR_ORDER_NOT_OPEN".to_string())
    );

    let (status, headers, done) = api
        .call_with("POST", &format!("{base}/complete"), None, &[])
        .await;
    assert_eq!(status, StatusCode::OK, "{done}");
    assert_eq!(headers["etag"], "\"2\"");
    assert_eq!(done["status"], "completed");
    assert!(done["closed_at"].is_string());
    assert_eq!(done["started_at"], started["started_at"]);

    // final: nothing more can be done
    for (uri, body) in [
        (format!("{base}/start"), None),
        (format!("{base}/complete"), None),
    ] {
        let (status, _) = api.failure("POST", &uri, body).await;
        assert_eq!(status, 409, "{uri}");
    }
    assert_eq!(
        api.failure(
            "POST",
            &format!("{base}/cancel"),
            Some(json!({"reason": "late"}))
        )
        .await,
        (409, "REPAIR_ORDER_CLOSED".to_string())
    );
    assert_eq!(
        api.failure("PATCH", &base, Some(json!({"due_date": null})))
            .await,
        (409, "REPAIR_ORDER_CLOSED".to_string())
    );
    let (_, _, saved) = api.call_with("GET", &base, None, &[]).await;
    assert_eq!(
        (saved["version"].as_i64(), saved["status"].as_str()),
        (Some(2), Some("completed"))
    );
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn an_order_can_be_cancelled_open_or_in_progress_with_a_reason_that_is_kept() {
    let api = setup().await;
    let set = api.set_with("6", Some(10.0)).await;
    let one = api.order(&set, "6a").await;
    let two = api.order(&set, "6b").await;
    let (a, b) = (
        format!("/repair-orders/{}", id(&one)),
        format!("/repair-orders/{}", id(&two)),
    );
    for bad in [
        json!({"reason": ""}),
        json!({"reason": "   "}),
        json!({"reason": "x".repeat(501)}),
    ] {
        let (status, code) = api.failure("POST", &format!("{a}/cancel"), Some(bad)).await;
        assert_eq!(status, 400, "{code}");
        assert!(
            code == "MISSING_FIELD" || code == "CANCELLATION_REASON_TOO_LONG",
            "{code}"
        );
    }
    let (status, cancelled) = api
        .call(
            "POST",
            &format!("{a}/cancel"),
            Some(json!({"reason": "  part is discontinued  "})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{cancelled}");
    assert_eq!(cancelled["status"], "cancelled");
    assert_eq!(cancelled["cancellation_reason"], "part is discontinued");
    assert!(cancelled["closed_at"].is_string());

    api.call("POST", &format!("{b}/start"), None).await;
    let (status, cancelled) = api
        .call(
            "POST",
            &format!("{b}/cancel"),
            Some(json!({"reason": "customer left"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{cancelled}");
    assert_eq!(cancelled["status"], "cancelled");
    assert!(cancelled["started_at"].is_string());

    // a cancelled order is final, and keeps its reason when read again
    assert_eq!(
        api.failure("POST", &format!("{a}/start"), None).await,
        (409, "REPAIR_ORDER_NOT_OPEN".to_string())
    );
    let (_, again) = api.call("GET", &a, None).await;
    assert_eq!(again["cancellation_reason"], "part is discontinued");
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_due_date_starts_from_the_quote_and_can_be_changed_or_cleared() {
    let api = setup().await;
    let set = api.set_with("7", Some(10.0)).await;
    let (_, quote) = api
        .call(
            "POST",
            "/quotes",
            Some(json!({"branch_id": set.branch, "customer_id": set.customer,
                        "equipment_id": set.machine, "items": [line("part", 1.0, 100)]})),
        )
        .await;
    let quote_id = quote["id"].as_str().unwrap().to_string();
    let today = quote["created_at"].as_str().unwrap()[..10].to_string();
    let promise =
        chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d").unwrap() + chrono::Days::new(10);
    let (status, _) = api
        .call(
            "PATCH",
            &format!("/quotes/{quote_id}"),
            Some(json!({"promise_date": promise.to_string()})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    api.call("POST", &format!("/quotes/{quote_id}/accept"), None)
        .await;
    let (_, order) = api.open_from(&quote_id, json!({})).await;
    assert_eq!(order["due_date"], promise.to_string());

    let base = format!("/repair-orders/{}", id(&order));
    let later = promise + chrono::Days::new(5);
    let (status, headers, changed) = api
        .call_with(
            "PATCH",
            &base,
            Some(json!({"due_date": later.to_string()})),
            &[],
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{changed}");
    assert_eq!(headers["etag"], "\"1\"");
    assert_eq!(changed["due_date"], later.to_string());
    let (status, cleared) = api
        .call("PATCH", &base, Some(json!({"due_date": null})))
        .await;
    assert_eq!(status, StatusCode::OK, "{cleared}");
    assert_eq!(cleared["due_date"], Value::Null);
    // before the day the order was opened
    assert_eq!(
        api.failure("PATCH", &base, Some(json!({"due_date": "2000-01-01"})))
            .await,
        (400, "DUE_DATE_BEFORE_OPENED".to_string())
    );
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn if_match_refuses_a_change_made_from_an_old_version_and_changes_nothing() {
    let api = setup().await;
    let set = api.set_with("8", Some(10.0)).await;
    let order = api.order(&set, "8").await;
    let base = format!("/repair-orders/{}", id(&order));

    let (status, _, _) = api
        .call_with(
            "POST",
            &format!("{base}/start"),
            None,
            &[("if-match", "\"0\"")],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    // an old screen (version 0) tries to cancel
    for stale in ["\"0\"", "0"] {
        let (status, _, answer) = api
            .call_with(
                "POST",
                &format!("{base}/cancel"),
                Some(json!({"reason": "late"})),
                &[("if-match", stale)],
            )
            .await;
        assert_eq!(status, StatusCode::CONFLICT, "{answer}");
        assert_eq!(answer["code"], "REPAIR_ORDER_CHANGED_MEANWHILE");
    }
    let (_, _, now) = api.call_with("GET", &base, None, &[]).await;
    assert_eq!(
        (now["status"].as_str(), now["version"].as_i64()),
        (Some("in_progress"), Some(1))
    );
    // the right version, and `*`, go through
    let (status, _, _) = api
        .call_with(
            "PATCH",
            &base,
            Some(json!({"due_date": null})),
            &[("if-match", "1")],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = api
        .call_with(
            "POST",
            &format!("{base}/complete"),
            None,
            &[("if-match", "*")],
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_list_is_newest_first_filtered_and_paged() {
    let api = setup().await;
    let one = api.set_with("9", Some(10.0)).await;
    let two = api.set_with("10", Some(10.0)).await;
    let first = api.order(&one, "a").await;
    let second = api.order(&one, "b").await;
    let third = api.order(&two, "c").await;
    api.call(
        "POST",
        &format!("/repair-orders/{}/start", id(&third)),
        None,
    )
    .await;

    let numbers = |list: &Value| -> Vec<String> {
        list.as_array()
            .unwrap()
            .iter()
            .map(|o| o["number"].as_str().unwrap().to_string())
            .collect()
    };
    let n = |o: &Value| o["number"].as_str().unwrap().to_string();
    let (status, mine) = api
        .call(
            "GET",
            &format!("/repair-orders?customer_id={}", one.customer),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(numbers(&mine), [n(&second), n(&first)]);
    assert_eq!(mine[0]["items"].as_array().unwrap().len(), 2); // each comes with its quote's lines
    let (_, at_branch) = api
        .call(
            "GET",
            &format!("/repair-orders?branch_id={}", two.branch),
            None,
        )
        .await;
    assert_eq!(numbers(&at_branch), [n(&third)]);
    let (_, for_machine) = api
        .call(
            "GET",
            &format!("/repair-orders?equipment_id={}", one.machine),
            None,
        )
        .await;
    assert_eq!(numbers(&for_machine), [n(&second), n(&first)]);
    let (_, started) = api
        .call(
            "GET",
            &format!(
                "/repair-orders?customer_id={}&status=in_progress",
                two.customer
            ),
            None,
        )
        .await;
    assert_eq!(numbers(&started), [n(&third)]);
    let (_, none) = api
        .call(
            "GET",
            &format!(
                "/repair-orders?customer_id={}&status=completed",
                two.customer
            ),
            None,
        )
        .await;
    assert!(none.as_array().unwrap().is_empty());
    let (_, page) = api
        .call(
            "GET",
            &format!(
                "/repair-orders?customer_id={}&limit=1&offset=1",
                one.customer
            ),
            None,
        )
        .await;
    assert_eq!(numbers(&page), [n(&first)]);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn eight_people_opening_one_quotes_repair_order_at_once_one_wins_and_the_rest_are_told() {
    let api = setup().await;
    let set = api.set_with("11", Some(10.0)).await;
    let quote = api.accepted(&set).await;
    let mut jobs = Vec::new();
    for _ in 0..8 {
        let app = api.app.clone();
        let uri = format!("/quotes/{quote}/repair-order");
        jobs.push(tokio::spawn(async move {
            use axum::body::Body;
            use axum::http::{Request, header};
            use tower::ServiceExt;
            let request = Request::builder()
                .method("POST")
                .uri(uri)
                .header(header::AUTHORIZATION, format!("Bearer {KEY}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("{}"))
                .unwrap();
            app.oneshot(request).await.unwrap().status()
        }));
    }
    let mut statuses = Vec::new();
    for job in jobs {
        statuses.push(job.await.unwrap().as_u16());
    }
    statuses.sort();
    assert_eq!(statuses, [201, 409, 409, 409, 409, 409, 409, 409]);
    let (_, all) = api
        .call(
            "GET",
            &format!("/repair-orders?customer_id={}", set.customer),
            None,
        )
        .await;
    assert_eq!(all.as_array().unwrap().len(), 1);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn repair_order_calls_log_nothing_about_the_order() {
    let api = setup().await;
    let log = LogText::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(log.clone())
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let set = api.set_with("12", Some(7777.25)).await;
    let quote = api.accepted(&set).await;
    let (_, order) = api
        .open_from(&quote, json!({"meter_reading": 8888.5}))
        .await;
    let order_id = id(&order).to_string();
    api.call("GET", &format!("/repair-orders/{order_id}"), None)
        .await;
    api.call("GET", "/repair-orders", None).await;
    api.call(
        "PATCH",
        &format!("/repair-orders/{order_id}"),
        Some(json!({"complaint": "Zed secret complaint", "cause": "Zed secret cause", "correction": "Zed secret correction"})),
    )
    .await;
    api.call(
        "PATCH",
        &format!("/repair-orders/{order_id}"),
        Some(json!({"due_date": "2999-05-06"})),
    )
    .await;
    api.call("POST", &format!("/repair-orders/{order_id}/start"), None)
        .await;
    api.call(
        "POST",
        &format!("/repair-orders/{order_id}/cancel"),
        Some(json!({"reason": "Zed secret reason"})),
    )
    .await;

    // (The order's own number, like a quote's, is a business reference and may appear in a
    // debug line; nothing else about the order may.)
    let text = log.0.lock().unwrap().clone();
    let text = String::from_utf8_lossy(&text).to_string();
    for secret in [
        order_id.as_str(),
        quote.as_str(),
        set.customer.as_str(),
        set.machine.as_str(),
        "Zed secret reason",
        "secret complaint",
        "secret cause",
        "secret correction",
        "8888.5",
        "7777.25",
        "2999-05-06",
        "API-T-Repairer",
        "API-T-R12",
        KEY,
    ] {
        assert!(
            !text.contains(secret),
            "{secret} leaked into the log:\n{text}"
        );
    }
    let repair_order_calls = text
        .lines()
        .filter(|l| l.contains("api_request") && l.contains("repair-order"))
        .count();
    assert_eq!(repair_order_calls, 7, "{text}"); // one line per call
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_complaint_cause_and_correction_are_written_changed_cleared_and_frozen_at_the_end() {
    let api = setup().await;
    let set = api.set_with("13", Some(10.0)).await;
    let order = api.order(&set, "13").await;
    let base = format!("/repair-orders/{}", id(&order));
    assert_eq!(
        (&order["complaint"], &order["cause"], &order["correction"]),
        (&Value::Null, &Value::Null, &Value::Null)
    );

    // any of them, alone or together; trimmed; new lines kept
    let (status, headers, saved) = api
        .call_with(
            "PATCH",
            &base,
            Some(json!({"complaint": "  Slow when warm.\nAlso noisy.  ", "cause": "Worn seal"})),
            &[],
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(headers["etag"], "\"1\"");
    assert_eq!(saved["complaint"], "Slow when warm.\nAlso noisy.");
    assert_eq!(saved["cause"], "Worn seal");
    assert_eq!(saved["correction"], Value::Null);

    // the due date and a paragraph in one call
    let (status, saved) = api
        .call(
            "PATCH",
            &base,
            Some(json!({"correction": "Replaced the seal.", "due_date": "2999-01-02"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(
        (saved["correction"].as_str(), saved["due_date"].as_str()),
        (Some("Replaced the seal."), Some("2999-01-02"))
    );
    assert_eq!(saved["cause"], "Worn seal"); // what was left out stays

    // too long, or a control character: nothing at all changes, not even the part that was fine
    for (bad, code) in [
        (
            json!({"cause": "x".repeat(4001)}),
            "REPAIR_ORDER_TEXT_TOO_LONG",
        ),
        (
            json!({"complaint": "bell\u{7}"}),
            "REPAIR_ORDER_TEXT_INVALID",
        ),
    ] {
        let mut body = bad.clone();
        body["correction"] = json!("this part is fine but must not be saved");
        assert_eq!(
            api.failure("PATCH", &base, Some(body)).await,
            (400, code.to_string())
        );
    }
    let (_, _, same) = api.call_with("GET", &base, None, &[]).await;
    assert_eq!(
        (same["version"].as_i64(), same["correction"].as_str()),
        (Some(2), Some("Replaced the seal."))
    );

    // blank and null clear
    let (_, cleared) = api
        .call(
            "PATCH",
            &base,
            Some(json!({"complaint": "   ", "cause": null})),
        )
        .await;
    assert_eq!(
        (&cleared["complaint"], &cleared["cause"]),
        (&Value::Null, &Value::Null)
    );
    assert_eq!(cleared["correction"], "Replaced the seal.");

    // an old screen is refused with If-Match, as always
    let (status, _, answer) = api
        .call_with(
            "PATCH",
            &base,
            Some(json!({"cause": "late"})),
            &[("if-match", "0")],
        )
        .await;
    assert_eq!(
        (status, answer["code"].as_str()),
        (StatusCode::CONFLICT, Some("REPAIR_ORDER_CHANGED_MEANWHILE"))
    );

    // the list carries them; and once completed (or cancelled) they can't change
    let (_, list) = api
        .call(
            "GET",
            &format!("/repair-orders?customer_id={}", set.customer),
            None,
        )
        .await;
    assert_eq!(list[0]["correction"], "Replaced the seal.");
    api.call("POST", &format!("{base}/start"), None).await;
    api.call("POST", &format!("{base}/complete"), None).await;
    assert_eq!(
        api.failure("PATCH", &base, Some(json!({"cause": "after the end"})))
            .await,
        (409, "REPAIR_ORDER_CLOSED".to_string())
    );
    let (_, end) = api.call("GET", &base, None).await;
    assert_eq!(
        (&end["cause"], end["correction"].as_str()),
        (&Value::Null, Some("Replaced the seal."))
    );
    api.finish().await;
}
