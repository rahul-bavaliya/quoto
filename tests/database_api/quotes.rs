// tests/database_api/quotes.rs: the quote calls with a real database behind them
//
// Branches, customers and machines here start with "API-T-" (a quote is found by its branch), and
// the tests remove their own quotes, then give back the numbers they used (see `support::clean`).

use super::support::{Api, setup};
use axum::http::StatusCode;
use sea_orm::ConnectionTrait;
use serde_json::{Value, json};

fn machine(serial: &str, owner: Option<&str>) -> Value {
    json!({
        "make": "John Deere", "model": "8R 310", "model_year": 2021,
        "serial_number": serial, "plate": "TR-1234",
        "meter_reading": 1250.5, "meter_type": "hr", "equipment_type": "tractor",
        "customer_id": owner
    })
}

fn line(kind: &str, quantity: f64, price: i64) -> Value {
    json!({"description": format!("API-T {kind}"), "quantity": quantity, "unit_price_cents": price, "kind": kind})
}

/// A branch, a customer and a machine that customer owns.
struct Set {
    branch: String,
    customer: String,
    machine: String,
}

impl Api {
    async fn machine_for(&self, serial: &str, owner: Option<&str>) -> String {
        let (status, body) = self
            .call("POST", "/equipment", Some(machine(serial, owner)))
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body["id"].as_str().unwrap().to_string()
    }

    async fn set(&self, tag: &str) -> Set {
        let branch = self.create_branch(&format!("API-T-Q{tag}")).await;
        let customer = self.create_customer(&format!("API-T-Quoter {tag}")).await;
        let machine = self
            .machine_for(&format!("API-T-Q{tag}"), Some(&customer))
            .await;
        Set {
            branch,
            customer,
            machine,
        }
    }

    async fn quick(&self, branch: &str) -> Value {
        let (status, body) = self
            .call("POST", "/quotes", Some(json!({"branch_id": branch})))
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body
    }

    async fn full(&self, set: &Set, items: Vec<Value>) -> Value {
        let (status, body) = self
            .call(
                "POST",
                "/quotes",
                Some(json!({
                    "branch_id": set.branch, "customer_id": set.customer,
                    "equipment_id": set.machine, "items": items
                })),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body
    }

    async fn failure(&self, method: &str, uri: &str, body: Option<Value>) -> (u16, String) {
        let (status, answer) = self.call(method, uri, body).await;
        (
            status.as_u16(),
            answer["code"].as_str().unwrap_or("-").to_string(),
        )
    }
}

fn id(quote: &Value) -> &str {
    quote["id"].as_str().unwrap()
}

fn counter(quote: &Value) -> u32 {
    quote["number"].as_str().unwrap()[7..].parse().unwrap()
}

// ===== create and read =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_quick_quote_is_pending_numbered_and_has_only_its_branch() {
    let api = setup().await;
    let branch = api.create_branch("API-T-Q1").await;
    let quote = api.quick(&branch).await;

    let year = chrono::Local::now().format("%Y").to_string();
    let number = quote["number"].as_str().unwrap();
    assert!(
        number.starts_with(&format!("Q-{year}-")) && number.len() == 15,
        "{number}"
    );
    assert_eq!(quote["status"], "pending");
    assert_eq!(quote["rejection_reason"], Value::Null);
    assert_eq!(quote["expired"], false);
    assert_eq!(quote["customer"], Value::Null);
    assert_eq!(quote["equipment"], Value::Null);
    assert_eq!(quote["promise_date"], Value::Null);
    assert_eq!(quote["items"], json!([]));
    assert_eq!(quote["branch"]["id"], branch);
    assert_eq!(quote["totals"]["total_cents"], 0);
    // good for 30 days from its own date
    let made =
        chrono::NaiveDate::parse_from_str(&quote["created_at"].as_str().unwrap()[..10], "%Y-%m-%d")
            .unwrap();
    assert_eq!(
        quote["expires_on"],
        (made + chrono::Days::new(30)).to_string()
    );

    let (status, again) = api
        .call("GET", &format!("/quotes/{}", id(&quote)), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(again, quote);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_whole_quote_keeps_its_parties_its_lines_in_order_and_adds_up_with_the_provinces_taxes() {
    let api = setup().await;
    let set = api.set("2").await;
    let quote = api
        .full(
            &set,
            vec![
                line("labour", 2.0, 10_000),
                line("part", 1.0, 5_000),
                line("fee", 1.0, 1_000),
                line("credit", 1.0, 500),
            ],
        )
        .await;
    assert_eq!(quote["customer"]["id"], set.customer);
    assert_eq!(quote["equipment"]["id"], set.machine);
    assert_eq!(quote["equipment"]["customer_id"], set.customer);
    let kinds: Vec<&str> = quote["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["labour", "part", "fee", "credit"]);
    let credit = &quote["items"][3];
    assert_eq!(credit["unit_price_cents"], 500); // as entered
    assert_eq!(credit["line_total_cents"], -500); // a credit takes away
    assert_eq!(credit["id"].as_str().unwrap().len(), 36);
    // Saskatchewan: 5% GST and 6% PST on 25,500
    assert_eq!(
        quote["totals"],
        json!({
            "labour_cents": 20_000, "parts_cents": 5_000, "fees_cents": 1_000, "credit_cents": -500,
            "subtotal_cents": 25_500, "gst_cents": 1_275, "pst_cents": 1_530, "total_cents": 28_305
        })
    );
    let (_, again) = api
        .call("GET", &format!("/quotes/{}", id(&quote)), None)
        .await;
    assert_eq!(again, quote);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_quote_nobody_has_is_a_404() {
    let api = setup().await;
    let nil = "00000000-0000-0000-0000-000000000000";
    assert_eq!(
        api.failure("GET", &format!("/quotes/{nil}"), None).await,
        (404, "UNKNOWN_QUOTE".into())
    );
    for (method, uri, body) in [
        (
            "PATCH",
            format!("/quotes/{nil}"),
            Some(json!({"expires_on": "2099-01-01"})),
        ),
        (
            "POST",
            format!("/quotes/{nil}/lines"),
            Some(line("part", 1.0, 1)),
        ),
        ("POST", format!("/quotes/{nil}/accept"), None),
        (
            "POST",
            format!("/quotes/{nil}/reject"),
            Some(json!({"reason": "x"})),
        ),
        ("POST", format!("/quotes/{nil}/reopen"), None),
    ] {
        assert_eq!(
            api.failure(method, &uri, body).await,
            (404, "UNKNOWN_QUOTE".into()),
            "{uri}"
        );
    }
    api.finish().await;
}

// ===== numbers and refusals =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn numbers_count_up_and_a_refused_quote_does_not_use_one_up() {
    let api = setup().await;
    let set = api.set("3").await;
    let first = api.quick(&set.branch).await;
    // refused in two different ways, after the number would have been taken
    let (status, _) = api
        .call(
            "POST",
            "/quotes",
            Some(json!({"branch_id": set.branch, "items": [line("credit", 1.0, 100)]})),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let nil = "00000000-0000-0000-0000-000000000000";
    assert_eq!(
        api.failure(
            "POST",
            "/quotes",
            Some(json!({"branch_id": set.branch, "customer_id": nil}))
        )
        .await,
        (404, "UNKNOWN_CUSTOMER".into())
    );
    let second = api.quick(&set.branch).await;
    assert_eq!(counter(&second), counter(&first) + 1);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_quote_is_refused_when_a_party_is_missing_inactive_or_does_not_match() {
    let api = setup().await;
    let set = api.set("4").await;
    let other = api.create_customer("API-T-Quoter Other").await;
    let nil = "00000000-0000-0000-0000-000000000000";
    let archived = api.machine_for("API-T-Q4-OLD", Some(&set.customer)).await;
    api.call("DELETE", &format!("/equipment/{archived}"), None)
        .await;
    let closed = api.create_customer("API-T-Quoter Closed").await;
    api.call(
        "PATCH",
        &format!("/customers/{closed}/status"),
        Some(json!({"status": "inactive"})),
    )
    .await;
    let shut = api.create_branch("API-T-Q4-SHUT").await;
    api.call(
        "PATCH",
        &format!("/branches/{shut}/status"),
        Some(json!({"status": "inactive"})),
    )
    .await;

    let cases = [
        (json!({"branch_id": nil}), (404, "UNKNOWN_BRANCH")),
        (json!({"branch_id": shut}), (409, "BRANCH_INACTIVE")),
        (
            json!({"branch_id": set.branch, "customer_id": closed}),
            (409, "CUSTOMER_INACTIVE"),
        ),
        (
            json!({"branch_id": set.branch, "customer_id": set.customer, "equipment_id": nil}),
            (404, "UNKNOWN_EQUIPMENT"),
        ),
        (
            json!({"branch_id": set.branch, "customer_id": set.customer, "equipment_id": archived}),
            (409, "EQUIPMENT_INACTIVE"),
        ),
        (
            json!({"branch_id": set.branch, "equipment_id": set.machine}),
            (409, "EQUIPMENT_NEEDS_CUSTOMER"),
        ),
        (
            json!({"branch_id": set.branch, "customer_id": other, "equipment_id": set.machine}),
            (409, "EQUIPMENT_NOT_OWNED_BY_CUSTOMER"),
        ),
    ];
    for (body, (status, code)) in cases {
        assert_eq!(
            api.failure("POST", "/quotes", Some(body.clone())).await,
            (status, code.into()),
            "{body}"
        );
    }
    // none of them was saved
    let (_, all) = api
        .call("GET", &format!("/quotes?branch_id={}", set.branch), None)
        .await;
    assert_eq!(all, json!([]));
    api.finish().await;
}

// ===== lines =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_line_is_added_at_the_end_and_the_old_lines_stay_exactly_as_they_were() {
    let api = setup().await;
    let set = api.set("5").await;
    let quote = api
        .full(
            &set,
            vec![line("labour", 1.0, 10_000), line("part", 2.0, 2_500)],
        )
        .await;
    let (status, after) = api
        .call(
            "POST",
            &format!("/quotes/{}/lines", id(&quote)),
            Some(line("fee", 1.0, 700)),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{after}");
    assert_eq!(after["items"].as_array().unwrap().len(), 3);
    assert_eq!(after["items"][0], quote["items"][0]);
    assert_eq!(after["items"][1], quote["items"][1]);
    assert_eq!(after["items"][2]["kind"], "fee");
    assert_eq!(after["totals"]["subtotal_cents"], 15_700);
    let (_, saved) = api
        .call("GET", &format!("/quotes/{}", id(&quote)), None)
        .await;
    assert_eq!(saved, after);

    // a credit bigger than everything so far is refused, and nothing changes
    assert_eq!(
        api.failure(
            "POST",
            &format!("/quotes/{}/lines", id(&quote)),
            Some(line("credit", 1.0, 99_999))
        )
        .await,
        (409, "CREDIT_TOO_LARGE".into())
    );
    // a wrong line is refused and nothing changes
    assert_eq!(
        api.failure(
            "POST",
            &format!("/quotes/{}/lines", id(&quote)),
            Some(json!({"description": "x", "quantity": 1, "unit_price_cents": 1, "kind": "gift"}))
        )
        .await,
        (400, "INVALID_LINE_KIND".into())
    );
    let (_, still) = api
        .call("GET", &format!("/quotes/{}", id(&quote)), None)
        .await;
    assert_eq!(still, after);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_quote_is_full_at_200_lines() {
    let api = setup().await;
    let branch = api.create_branch("API-T-Q6").await;
    let hundred: Vec<Value> = (0..100).map(|_| json!({"description": "Bolt", "quantity": 1, "unit_price_cents": 1, "kind": "part"})).collect();
    let (status, quote) = api
        .call(
            "POST",
            "/quotes",
            Some(json!({"branch_id": branch, "items": hundred})),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{quote}");
    for _ in 0..100 {
        let (status, _) = api
            .call(
                "POST",
                &format!("/quotes/{}/lines", id(&quote)),
                Some(line("part", 1.0, 1)),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED);
    }
    assert_eq!(
        api.failure(
            "POST",
            &format!("/quotes/{}/lines", id(&quote)),
            Some(line("part", 1.0, 1))
        )
        .await,
        (409, "TOO_MANY_LINES".into())
    );
    let (_, saved) = api
        .call("GET", &format!("/quotes/{}", id(&quote)), None)
        .await;
    assert_eq!(saved["items"].as_array().unwrap().len(), 200);
    api.finish().await;
}

// ===== dates =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_dates_change_together_or_not_at_all() {
    let api = setup().await;
    let branch = api.create_branch("API-T-Q7").await;
    let quote = api.quick(&branch).await;
    let uri = format!("/quotes/{}", id(&quote));

    let (status, after) = api
        .call("PATCH", &uri, Some(json!({"promise_date": "2099-06-01"})))
        .await;
    assert_eq!(status, StatusCode::OK, "{after}");
    assert_eq!(after["promise_date"], "2099-06-01");
    assert_eq!(after["expires_on"], quote["expires_on"]); // left out = untouched

    let (_, both) = api
        .call(
            "PATCH",
            &uri,
            Some(json!({"expires_on": "2099-12-31", "promise_date": "2099-07-01"})),
        )
        .await;
    assert_eq!(
        (&both["expires_on"], &both["promise_date"]),
        (&json!("2099-12-31"), &json!("2099-07-01"))
    );

    // one good date and one before the quote's own date: refused as a whole
    let (status, refused) = api
        .call(
            "PATCH",
            &uri,
            Some(json!({"expires_on": "2098-01-01", "promise_date": "2000-01-01"})),
        )
        .await;
    assert_eq!(
        (status, refused["code"].as_str()),
        (StatusCode::BAD_REQUEST, Some("PROMISE_BEFORE_QUOTE_DATE"))
    );
    assert_eq!(
        api.failure("PATCH", &uri, Some(json!({"expires_on": "2000-01-01"})))
            .await,
        (400, "EXPIRY_BEFORE_QUOTE_DATE".into())
    );
    let (_, still) = api.call("GET", &uri, None).await;
    assert_eq!(still, both);
    api.finish().await;
}

// ===== the customer and the machine, added later =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_customer_then_the_machine_can_be_added_once_each() {
    let api = setup().await;
    let set = api.set("8").await;
    let stranger = api.create_customer("API-T-Quoter Stranger").await;
    let strangers_machine = api.machine_for("API-T-Q8-S", Some(&stranger)).await;
    let nil = "00000000-0000-0000-0000-000000000000";
    let quote = api.quick(&set.branch).await;
    let base = format!("/quotes/{}", id(&quote));

    // the machine needs the customer first
    assert_eq!(
        api.failure(
            "PATCH",
            &format!("{base}/equipment"),
            Some(json!({"equipment_id": set.machine}))
        )
        .await,
        (409, "EQUIPMENT_NEEDS_CUSTOMER".into())
    );
    assert_eq!(
        api.failure(
            "PATCH",
            &format!("{base}/customer"),
            Some(json!({"customer_id": nil}))
        )
        .await,
        (404, "UNKNOWN_CUSTOMER".into())
    );
    let (status, with_customer) = api
        .call(
            "PATCH",
            &format!("{base}/customer"),
            Some(json!({"customer_id": set.customer})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{with_customer}");
    assert_eq!(with_customer["customer"]["id"], set.customer);
    // one customer only
    assert_eq!(
        api.failure(
            "PATCH",
            &format!("{base}/customer"),
            Some(json!({"customer_id": stranger}))
        )
        .await,
        (409, "ALREADY_ATTACHED".into())
    );
    // the machine must be theirs, and must exist
    assert_eq!(
        api.failure(
            "PATCH",
            &format!("{base}/equipment"),
            Some(json!({"equipment_id": strangers_machine}))
        )
        .await,
        (409, "EQUIPMENT_NOT_OWNED_BY_CUSTOMER".into())
    );
    assert_eq!(
        api.failure(
            "PATCH",
            &format!("{base}/equipment"),
            Some(json!({"equipment_id": nil}))
        )
        .await,
        (404, "UNKNOWN_EQUIPMENT".into())
    );
    let (status, with_machine) = api
        .call(
            "PATCH",
            &format!("{base}/equipment"),
            Some(json!({"equipment_id": set.machine})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{with_machine}");
    assert_eq!(with_machine["equipment"]["id"], set.machine);
    assert_eq!(
        api.failure(
            "PATCH",
            &format!("{base}/equipment"),
            Some(json!({"equipment_id": set.machine}))
        )
        .await,
        (409, "ALREADY_ATTACHED".into())
    );
    let (_, saved) = api.call("GET", &base, None).await;
    assert_eq!(saved, with_machine);

    // a customer who is closed can't be added
    let closed = api.create_customer("API-T-Quoter Closed").await;
    api.call(
        "PATCH",
        &format!("/customers/{closed}/status"),
        Some(json!({"status": "inactive"})),
    )
    .await;
    let another = api.quick(&set.branch).await;
    assert_eq!(
        api.failure(
            "PATCH",
            &format!("/quotes/{}/customer", id(&another)),
            Some(json!({"customer_id": closed}))
        )
        .await,
        (409, "CUSTOMER_INACTIVE".into())
    );
    api.finish().await;
}

// ===== a quote keeps what was true that day =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn changing_the_customer_machine_or_branch_later_never_changes_the_quote() {
    let api = setup().await;
    let set = api.set("9").await;
    let quote = api.full(&set, vec![line("labour", 1.0, 10_000)]).await;
    let uri = format!("/quotes/{}", id(&quote));

    api.call(
        "PATCH",
        &format!("/customers/{}", set.customer),
        Some(json!({"name": "API-T-Renamed Person", "city": "Calgary"})),
    )
    .await;
    api.call(
        "PATCH",
        &format!("/equipment/{}", set.machine),
        Some(json!({"make": "Case IH", "plate": null})),
    )
    .await;
    api.call(
        "PATCH",
        &format!("/branches/{}", set.branch),
        Some(json!({"name": "Renamed Branch"})),
    )
    .await;
    let (_, now) = api
        .call("GET", &format!("/customers/{}", set.customer), None)
        .await;
    assert_eq!(now["name"], "API-T-Renamed Person"); // the customer really changed
    let (_, saved) = api.call("GET", &uri, None).await;
    assert_eq!(saved, quote);

    // and the quote stays readable when all three are made inactive
    api.call(
        "PATCH",
        &format!("/branches/{}/status", set.branch),
        Some(json!({"status": "inactive"})),
    )
    .await;
    api.call(
        "PATCH",
        &format!("/customers/{}/status", set.customer),
        Some(json!({"status": "inactive"})),
    )
    .await;
    api.call("DELETE", &format!("/equipment/{}", set.machine), None)
        .await;
    let (status, saved) = api.call("GET", &uri, None).await;
    assert_eq!((status, &saved["items"]), (StatusCode::OK, &quote["items"]));
    api.finish().await;
}

// ===== decisions =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_quote_is_accepted_once_and_then_it_is_final() {
    let api = setup().await;
    let set = api.set("10").await;
    let quick = api.quick(&set.branch).await;
    // a repair order needs both the customer and the machine
    assert_eq!(
        api.failure("POST", &format!("/quotes/{}/accept", id(&quick)), None)
            .await,
        (409, "QUOTE_INCOMPLETE".into())
    );

    let quote = api.full(&set, vec![line("labour", 1.0, 10_000)]).await;
    let base = format!("/quotes/{}", id(&quote));
    let (status, accepted) = api.call("POST", &format!("{base}/accept"), None).await;
    assert_eq!(status, StatusCode::OK, "{accepted}");
    assert_eq!(accepted["status"], "accepted");
    let (_, saved) = api.call("GET", &base, None).await;
    assert_eq!(saved["status"], "accepted");

    for (uri, body, code) in [
        (format!("{base}/accept"), None, "ALREADY_DECIDED"),
        (
            format!("{base}/reject"),
            Some(json!({"reason": "late"})),
            "ALREADY_DECIDED",
        ),
        (format!("{base}/reopen"), None, "NOT_REJECTED"),
        (
            format!("{base}/lines"),
            Some(line("part", 1.0, 1)),
            "ALREADY_DECIDED",
        ),
    ] {
        assert_eq!(
            api.failure("POST", &uri, body).await,
            (409, code.into()),
            "{uri}"
        );
    }
    assert_eq!(
        api.failure(
            "PATCH",
            &format!("{base}/customer"),
            Some(json!({"customer_id": set.customer}))
        )
        .await,
        (409, "ALREADY_DECIDED".into())
    );
    let (_, last) = api.call("GET", &base, None).await;
    assert_eq!(last, saved);
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_rejection_keeps_its_reason_and_a_reopened_quote_forgets_it() {
    let api = setup().await;
    let set = api.set("11").await;
    let quote = api.full(&set, vec![line("labour", 1.0, 10_000)]).await;
    let base = format!("/quotes/{}", id(&quote));

    assert_eq!(
        api.failure(
            "POST",
            &format!("{base}/reject"),
            Some(json!({"reason": "   "}))
        )
        .await,
        (400, "MISSING_FIELD".into())
    );
    assert_eq!(
        api.failure(
            "POST",
            &format!("{base}/reject"),
            Some(json!({"reason": "x".repeat(501)}))
        )
        .await,
        (400, "REJECTION_REASON_TOO_LONG".into())
    );
    assert_eq!(
        api.failure("POST", &format!("{base}/reopen"), None).await,
        (409, "NOT_REJECTED".into())
    );

    let (status, rejected) = api
        .call(
            "POST",
            &format!("{base}/reject"),
            Some(json!({"reason": "  Too expensive  "})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{rejected}");
    assert_eq!(
        (&rejected["status"], &rejected["rejection_reason"]),
        (&json!("rejected"), &json!("Too expensive"))
    );
    let (_, saved) = api.call("GET", &base, None).await;
    assert_eq!(saved, rejected);
    assert_eq!(
        api.failure(
            "POST",
            &format!("{base}/reject"),
            Some(json!({"reason": "again"}))
        )
        .await,
        (409, "ALREADY_DECIDED".into())
    );
    assert_eq!(
        api.failure("POST", &format!("{base}/accept"), None).await,
        (409, "ALREADY_DECIDED".into())
    );

    let (status, reopened) = api.call("POST", &format!("{base}/reopen"), None).await;
    assert_eq!(status, StatusCode::OK, "{reopened}");
    assert_eq!(
        (&reopened["status"], &reopened["rejection_reason"]),
        (&json!("pending"), &Value::Null)
    );
    let (status, accepted) = api.call("POST", &format!("{base}/accept"), None).await;
    assert_eq!(
        (status, &accepted["status"]),
        (StatusCode::OK, &json!("accepted"))
    );
    api.finish().await;
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_quote_past_its_expiry_says_so_cannot_be_accepted_but_can_be_rejected_or_given_more_time()
{
    let api = setup().await;
    let set = api.set("12").await;
    let quote = api.full(&set, vec![line("labour", 1.0, 10_000)]).await;
    let base = format!("/quotes/{}", id(&quote));
    // the quote was made long ago (test housekeeping: the API itself can't date a quote back)
    api.db
        .execute_unprepared(&format!(
            "UPDATE quotes SET created_at = '2000-01-01 09:00:00', expires_on = '2000-01-31' WHERE id = '{}'",
            id(&quote)
        ))
        .await
        .unwrap();
    let (_, old) = api.call("GET", &base, None).await;
    assert_eq!(old["expired"], true);
    assert_eq!(
        api.failure("POST", &format!("{base}/accept"), None).await,
        (409, "QUOTE_EXPIRED".into())
    );
    // more time makes it good again
    let (_, extended) = api
        .call("PATCH", &base, Some(json!({"expires_on": "2099-01-01"})))
        .await;
    assert_eq!(
        (&extended["expired"], &extended["expires_on"]),
        (&json!(false), &json!("2099-01-01"))
    );
    let (status, accepted) = api.call("POST", &format!("{base}/accept"), None).await;
    assert_eq!(status, StatusCode::OK, "{accepted}");
    // and an expired one can still be turned down
    let other = api.full(&set, vec![line("part", 1.0, 100)]).await;
    api.db
        .execute_unprepared(&format!(
            "UPDATE quotes SET created_at = '2000-01-01 09:00:00', expires_on = '2000-01-31' WHERE id = '{}'",
            id(&other)
        ))
        .await
        .unwrap();
    let (status, rejected) = api
        .call(
            "POST",
            &format!("/quotes/{}/reject", id(&other)),
            Some(json!({"reason": "too late"})),
        )
        .await;
    assert_eq!(
        (status, &rejected["status"]),
        (StatusCode::OK, &json!("rejected"))
    );
    api.finish().await;
}

// ===== lists =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_list_is_newest_first_filtered_and_paged_and_shows_the_lines() {
    let api = setup().await;
    let one = api.set("13").await;
    let two = api.set("14").await;
    let a = api.full(&one, vec![line("labour", 1.0, 100)]).await;
    let b = api.quick(&one.branch).await;
    let c = api.full(&two, vec![line("part", 3.0, 200)]).await;

    let (status, mine) = api
        .call("GET", &format!("/quotes?branch_id={}", one.branch), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let ids: Vec<&str> = mine.as_array().unwrap().iter().map(id).collect();
    assert_eq!(ids, [id(&b), id(&a)]); // the newest first
    assert_eq!(mine[1]["items"].as_array().unwrap().len(), 1);

    let (_, by_customer) = api
        .call(
            "GET",
            &format!("/quotes?customer_id={}", two.customer),
            None,
        )
        .await;
    assert_eq!(
        by_customer
            .as_array()
            .unwrap()
            .iter()
            .map(id)
            .collect::<Vec<_>>(),
        [id(&c)]
    );
    let (_, by_machine) = api
        .call(
            "GET",
            &format!("/quotes?equipment_id={}", one.machine),
            None,
        )
        .await;
    assert_eq!(
        by_machine
            .as_array()
            .unwrap()
            .iter()
            .map(id)
            .collect::<Vec<_>>(),
        [id(&a)]
    );
    let (_, page_one) = api
        .call(
            "GET",
            &format!("/quotes?branch_id={}&limit=1", one.branch),
            None,
        )
        .await;
    let (_, page_two) = api
        .call(
            "GET",
            &format!("/quotes?branch_id={}&limit=1&offset=1", one.branch),
            None,
        )
        .await;
    assert_eq!((id(&page_one[0]), id(&page_two[0])), (id(&b), id(&a)));
    let (_, none) = api
        .call(
            "GET",
            &format!("/quotes?branch_id={}&limit=5&offset=9", one.branch),
            None,
        )
        .await;
    assert_eq!(none, json!([]));
    api.finish().await;
}

// ===== a damaged quote is refused, not shown =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_saved_quote_that_is_damaged_is_a_503_that_shows_nothing_of_it() {
    let api = setup().await;
    let set = api.set("15").await;
    let quote = api.full(&set, vec![line("labour", 1.0, 10_000)]).await;
    api.db
        .execute_unprepared(&format!(
            "UPDATE quotes SET status = 'maybe' WHERE id = '{}'",
            id(&quote)
        ))
        .await
        .unwrap();
    let (status, body) = api
        .call("GET", &format!("/quotes/{}", id(&quote)), None)
        .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Some("STORED_QUOTE_DAMAGED")
        )
    );
    assert!(!body.to_string().contains("maybe"));
    api.finish().await;
}

// ===== at the same moment =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn quotes_made_at_the_same_moment_get_different_numbers() {
    let api = setup().await;
    let branch = api.create_branch("API-T-Q16").await;
    let mut jobs = Vec::new();
    for _ in 0..6 {
        let app = api.app.clone();
        let branch = branch.clone();
        jobs.push(tokio::spawn(async move {
            use axum::body::{Body, to_bytes};
            use axum::http::{Request, header};
            use tower::ServiceExt;
            let request = Request::builder()
                .method("POST")
                .uri("/quotes")
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", super::support::KEY),
                )
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"branch_id": branch}).to_string()))
                .unwrap();
            let response = app.oneshot(request).await.unwrap();
            let status = response.status();
            let bytes = to_bytes(response.into_body(), 1 << 20).await.unwrap();
            (status, serde_json::from_slice::<Value>(&bytes).unwrap())
        }));
    }
    let mut numbers = Vec::new();
    for job in jobs {
        let (status, quote) = job.await.unwrap();
        assert_eq!(status, StatusCode::CREATED, "{quote}");
        numbers.push(counter(&quote));
    }
    numbers.sort_unstable();
    let first = numbers[0];
    assert_eq!(numbers, (first..first + 6).collect::<Vec<_>>()); // no repeat, no gap
    api.finish().await;
}
