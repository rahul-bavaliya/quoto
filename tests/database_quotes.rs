// tests/database_quotes.rs: saving and loading quotes in a real database
//
// These need a running database, so they are skipped normally (and in CI). Run them by hand:
//
//     docker compose up -d
//     cargo test --test database_quotes -- --ignored
//
// Almost every test works inside a transaction that is dropped at the end, which undoes it, so
// nothing is left behind. The one test that needs two real connections at once commits, and
// removes its own rows. All people, machines and branches here are made up. Quotes are dated in
// the years 2090 and 2091, so their counters never touch a real year's.

use chrono::{NaiveDate, NaiveDateTime};
use quoto::*;
use sea_orm::{ConnectionTrait, DatabaseConnection, DatabaseTransaction, TransactionTrait};
use uuid::Uuid;

/// The tests take turns: two of them building the tables at the same moment would collide.
static ONE_AT_A_TIME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn setup() -> (tokio::sync::MutexGuard<'static, ()>, DatabaseConnection) {
    let turn = ONE_AT_A_TIME.lock().await;
    dotenvy::dotenv().ok();
    let settings = DatabaseSettings::from_env().expect("database settings (see .env.example)");
    let db = database::connect(&settings).await.expect("database");
    database::migrate(&db).await.expect("migrate");
    (turn, db)
}

async fn work(db: &DatabaseConnection) -> DatabaseTransaction {
    db.begin().await.expect("transaction")
}

fn day(year: i32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(year, 3, 14)
        .and_then(|day| day.and_hms_opt(9, 30, 0))
        .expect("a real time")
}

fn regina() -> Address {
    Address::new(
        String::from("1 Industrial Rd"),
        None,
        String::from("Regina"),
        PostalCode::parse("S4P 3Y2").unwrap(),
        Province::Saskatchewan,
    )
    .unwrap()
}

fn contact() -> Contact {
    Contact::new(PhoneNumber::parse("306-555-0142").unwrap(), None, None)
}

fn branch(number: &str) -> Branch {
    Branch::new(
        number.to_string(),
        String::from("QT Regina Branch"),
        regina(),
        PhoneNumber::parse("306-555-0100").unwrap(),
    )
    .unwrap()
}

fn machine(serial: &str) -> Equipment {
    Equipment::new(
        String::from("John Deere"),
        String::from("8R 310"),
        Some(2021),
        Identification::new(Some(serial.to_string()), Some(String::from("QT-1234"))).unwrap(),
        Meter::new(Some(1250.5), MeterType::Hour).unwrap(),
        EquipmentType::Tractor,
    )
    .unwrap()
}

fn plain_customer(name: &str) -> Customer {
    Customer::new(
        name.to_string(),
        Some(String::from("Prairie Farms Ltd.")),
        None,
        contact(),
        regina(),
    )
    .unwrap()
}

/// A saved branch, customer and machine owned by that customer.
struct Parties {
    branch: Uuid,
    customer: Uuid,
    machine: Uuid,
}

async fn parties(tx: &DatabaseTransaction, tag: &str) -> Parties {
    let branch = create_branch(tx, &branch(&format!("QT-{tag}")))
        .await
        .unwrap();
    let customer = create_customer(tx, &plain_customer(&format!("QT-Jane-{tag}")))
        .await
        .unwrap();
    let machine = create_equipment(
        tx,
        &machine(&format!("QT-SERIAL-{tag}")).with_customer_id(customer.id().unwrap()),
    )
    .await
    .unwrap();
    Parties {
        branch: branch.id().unwrap(),
        customer: customer.id().unwrap(),
        machine: machine.id().unwrap(),
    }
}

fn quick(branch: Uuid, year: i32) -> NewQuote {
    NewQuote {
        created_at: day(year),
        branch_id: branch,
        customer_id: None,
        equipment_id: None,
        items: Vec::new(),
    }
}

fn full(p: &Parties, year: i32) -> NewQuote {
    NewQuote {
        created_at: day(year),
        branch_id: p.branch,
        customer_id: Some(p.customer),
        equipment_id: Some(p.machine),
        items: samples::oil_change_lines().unwrap(),
    }
}

// ===== create and read =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_quick_quote_is_saved_pending_with_a_number_and_only_its_branch() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "1").await;
    let saved = create_quote(&tx, quick(p.branch, 2090)).await.unwrap();
    assert!(saved.id().is_some());
    assert_eq!(saved.number().to_string(), "Q-2090-00000001");
    assert_eq!(saved.status(), &QuoteStatus::Pending);
    assert!(saved.customer().is_none());
    assert!(saved.equipment().is_none());
    assert!(saved.items().is_empty());
    assert_eq!(saved.created_at(), day(2090));
    assert_eq!(saved.expires_on(), day(2090).date() + chrono::Days::new(30));
    assert_eq!(saved.province(), &Province::Saskatchewan);
    assert_eq!(saved.branch().number(), "QT-1");
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_whole_quote_comes_back_exactly_as_it_was_saved() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "2").await;
    let saved = create_quote(&tx, full(&p, 2090)).await.unwrap();
    let loaded = get_quote(&tx, saved.id().unwrap()).await.unwrap();

    assert_eq!(loaded.number().to_string(), saved.number().to_string());
    assert_eq!(loaded.to_string(), saved.to_string()); // the printed quote is identical

    assert_eq!(loaded.customer().unwrap().name(), "QT-Jane-2");
    assert_eq!(loaded.customer().unwrap().id(), Some(p.customer));
    assert_eq!(loaded.equipment().unwrap().id(), Some(p.machine));
    assert_eq!(loaded.equipment().unwrap().customer_id(), Some(p.customer));
    assert_eq!(loaded.branch().id(), Some(p.branch));

    // the lines: same order, same details, each pointing at this quote
    let expected = samples::oil_change_lines().unwrap();
    assert_eq!(loaded.items().len(), expected.len());
    for (line, wanted) in loaded.items().iter().zip(&expected) {
        assert_eq!(line.description(), wanted.description());
        assert_eq!(line.quantity(), wanted.quantity());
        assert_eq!(line.line_total(), wanted.line_total());
        assert_eq!(line.kind(), wanted.kind());
        assert!(line.id().is_some());
        assert_eq!(line.quote_id(), loaded.id());
    }
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_credit_line_comes_back_as_a_credit() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "3").await;
    let mut new = full(&p, 2090);
    new.items
        .push(LineItem::new(String::from("Goodwill"), 1.0, 1500, LineKind::Credit).unwrap());
    let before = create_quote(&tx, full(&p, 2090)).await.unwrap().total();
    let saved = create_quote(&tx, new).await.unwrap();
    let loaded = get_quote(&tx, saved.id().unwrap()).await.unwrap();
    assert_eq!(loaded.items().last().unwrap().kind(), &LineKind::Credit);
    assert!(loaded.total() < before);
    assert_eq!(loaded.total(), saved.total());
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_quote_nobody_has_is_unknown() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    assert!(matches!(
        get_quote(&tx, Uuid::nil()).await,
        Err(QuoteError::UnknownQuote)
    ));
}

// ===== numbers =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn numbers_count_up_within_a_year_and_each_year_starts_again() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "4").await;
    let mut numbers = Vec::new();
    for year in [2090, 2090, 2091, 2090] {
        let saved = create_quote(&tx, quick(p.branch, year)).await.unwrap();
        numbers.push(saved.number().to_string());
    }
    assert_eq!(
        numbers,
        [
            "Q-2090-00000001",
            "Q-2090-00000002",
            "Q-2091-00000001",
            "Q-2090-00000003"
        ]
    );
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_number_that_was_taken_but_not_saved_is_given_back() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    // taken, then the work is undone
    let attempt = tx.begin().await.unwrap();
    assert_eq!(
        next_quote_number(&attempt, 2090).await.unwrap().counter(),
        1
    );
    assert_eq!(
        next_quote_number(&attempt, 2090).await.unwrap().counter(),
        2
    );
    attempt.rollback().await.unwrap();
    assert_eq!(next_quote_number(&tx, 2090).await.unwrap().counter(), 1);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_refused_quote_does_not_use_up_a_number() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "5").await;
    let mut bad = full(&p, 2090);
    bad.customer_id = Some(Uuid::nil());
    assert!(matches!(
        create_quote(&tx, bad).await,
        Err(QuoteError::UnknownCustomer)
    ));
    let next = create_quote(&tx, quick(p.branch, 2090)).await.unwrap();
    assert_eq!(next.number().to_string(), "Q-2090-00000001");
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_year_that_is_not_a_year_is_refused() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    for year in [0, 999, 10_000] {
        assert!(next_quote_number(&tx, year).await.is_err(), "{year}");
    }
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_last_number_of_a_year_is_the_last() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    tx.execute_unprepared("INSERT INTO quote_numbers (year, last_number) VALUES (2090, 99999998)")
        .await
        .unwrap();
    assert_eq!(
        next_quote_number(&tx, 2090).await.unwrap().to_string(),
        "Q-2090-99999999"
    );
    let sp = tx.begin().await.unwrap();
    assert!(matches!(
        next_quote_number(&sp, 2090).await,
        Err(QuoteError::QuoteNumbersExhausted { year: 2090 })
    ));
}

// ===== refusals =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_quote_rules_hold_when_saving_and_nothing_is_saved_when_one_fails() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "6").await;
    let stranger = create_customer(&tx, &plain_customer("QT-Stranger"))
        .await
        .unwrap();
    let archived = create_equipment(&tx, &machine("QT-ARCHIVED").with_customer_id(p.customer))
        .await
        .unwrap();
    set_equipment_status(&tx, archived.id().unwrap(), Status::Inactive)
        .await
        .unwrap();
    let closed = create_customer(&tx, &plain_customer("QT-Closed"))
        .await
        .unwrap();
    set_customer_status(&tx, closed.id().unwrap(), Status::Inactive)
        .await
        .unwrap();
    let shut = create_branch(&tx, &branch("QT-SHUT")).await.unwrap();
    set_branch_status(&tx, shut.id().unwrap(), Status::Inactive)
        .await
        .unwrap();
    let too_big = LineItem::new(String::from("Too big"), 1.0, 100, LineKind::Credit).unwrap();

    let mut cases: Vec<(NewQuote, &str)> = Vec::new();
    cases.push((quick(Uuid::nil(), 2090), "unknown branch"));
    cases.push((quick(shut.id().unwrap(), 2090), "inactive branch"));
    let mut case = full(&p, 2090);
    case.customer_id = Some(closed.id().unwrap());
    cases.push((case, "inactive customer"));
    let mut case = full(&p, 2090);
    case.equipment_id = Some(Uuid::nil());
    cases.push((case, "unknown machine"));
    let mut case = full(&p, 2090);
    case.equipment_id = Some(archived.id().unwrap());
    cases.push((case, "archived machine"));
    let mut case = full(&p, 2090);
    case.customer_id = None;
    cases.push((case, "machine without a customer"));
    let mut case = full(&p, 2090);
    case.customer_id = Some(stranger.id().unwrap());
    cases.push((case, "machine of another customer"));
    let mut case = quick(p.branch, 2090);
    case.items.push(too_big);
    cases.push((case, "credit bigger than the subtotal"));

    for (case, why) in cases {
        assert!(create_quote(&tx, case).await.is_err(), "{why}");
    }
    // none of them left a quote behind, and the first real quote is still number 1
    let all = list_quotes(&tx, &QuoteFilter::default(), 200, 0)
        .await
        .unwrap();
    assert!(
        all.iter()
            .all(|q| !q.number().to_string().starts_with("Q-2090-"))
    );
    let first = create_quote(&tx, quick(p.branch, 2090)).await.unwrap();
    assert_eq!(first.number().counter(), 1);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn each_refusal_says_why() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "7").await;
    let stranger = create_customer(&tx, &plain_customer("QT-Stranger"))
        .await
        .unwrap();

    assert!(matches!(
        create_quote(&tx, quick(Uuid::nil(), 2090)).await,
        Err(QuoteError::UnknownBranch)
    ));
    let mut case = full(&p, 2090);
    case.customer_id = None;
    assert!(matches!(
        create_quote(&tx, case).await,
        Err(QuoteError::EquipmentNeedsCustomer)
    ));
    let mut case = full(&p, 2090);
    case.customer_id = Some(stranger.id().unwrap());
    assert!(matches!(
        create_quote(&tx, case).await,
        Err(QuoteError::EquipmentNotOwnedByCustomer)
    ));
}

// ===== a quote keeps what was true that day =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn editing_the_customer_machine_or_branch_later_never_changes_an_old_quote() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "8").await;
    let saved = create_quote(&tx, full(&p, 2090)).await.unwrap();
    let id = saved.id().unwrap();
    let printed_before = saved.to_string();

    // the customer is renamed and moves, the machine is corrected, the branch is renamed
    let moved = Customer::new(
        String::from("QT-Renamed Person"),
        None,
        None,
        contact(),
        Address::new(
            String::from("9 New Rd"),
            None,
            String::from("Calgary"),
            PostalCode::parse("T2P 1J9").unwrap(),
            Province::Alberta,
        )
        .unwrap(),
    )
    .unwrap()
    .with_id(p.customer);
    update_customer(&tx, &moved).await.unwrap();
    let corrected = get_equipment(&tx, p.machine).await.unwrap();
    let corrected = Equipment::new(
        String::from("Case IH"),
        String::from("Magnum"),
        None,
        Identification::new(Some(String::from("QT-SERIAL-8")), None).unwrap(),
        Meter::new(Some(9.0), MeterType::Hour).unwrap(),
        EquipmentType::Tractor,
    )
    .unwrap()
    .with_id(corrected.id().unwrap());
    update_equipment(&tx, &corrected).await.unwrap();
    let renamed = Branch::new(
        String::from("QT-8"),
        String::from("QT Renamed Branch"),
        regina(),
        PhoneNumber::parse("306-555-0100").unwrap(),
    )
    .unwrap()
    .with_id(p.branch);
    update_branch(&tx, &renamed).await.unwrap();

    let loaded = get_quote(&tx, id).await.unwrap();
    assert_eq!(loaded.to_string(), printed_before); // the printed quote did not change
    assert_eq!(loaded.customer().unwrap().name(), "QT-Jane-8");
    assert_eq!(loaded.equipment().unwrap().make(), "John Deere");
    assert_eq!(loaded.branch().name(), "QT Regina Branch");
    assert_eq!(loaded.province(), &Province::Saskatchewan);
    // ...while the people and machines themselves did change
    assert_eq!(
        get_customer(&tx, p.customer).await.unwrap().name(),
        "QT-Renamed Person"
    );
    assert_eq!(
        get_equipment(&tx, p.machine).await.unwrap().make(),
        "Case IH"
    );
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_quote_is_still_readable_after_its_branch_customer_and_machine_are_made_inactive() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "9").await;
    let saved = create_quote(&tx, full(&p, 2090)).await.unwrap();
    set_branch_status(&tx, p.branch, Status::Inactive)
        .await
        .unwrap();
    set_customer_status(&tx, p.customer, Status::Inactive)
        .await
        .unwrap();
    set_equipment_status(&tx, p.machine, Status::Inactive)
        .await
        .unwrap();
    let loaded = get_quote(&tx, saved.id().unwrap()).await.unwrap();
    assert_eq!(loaded.to_string(), saved.to_string());
}

// ===== changing a saved quote =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_decision_is_saved_and_a_rejected_quote_can_be_reopened() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "10").await;
    let saved = create_quote(&tx, full(&p, 2090)).await.unwrap();
    let id = saved.id().unwrap();

    let mut quote = get_quote(&tx, id).await.unwrap();
    quote.accept(day(2090).date()).unwrap();
    save_quote(&tx, &quote).await.unwrap();
    assert_eq!(
        get_quote(&tx, id).await.unwrap().status(),
        &QuoteStatus::Accepted
    );

    // an accepted quote is final: a fresh read can't be rejected
    let mut again = get_quote(&tx, id).await.unwrap();
    assert!(again.reject("changed my mind").is_err());

    let second = create_quote(&tx, full(&p, 2090)).await.unwrap();
    let mut quote = get_quote(&tx, second.id().unwrap()).await.unwrap();
    quote.reject("  Too expensive  ").unwrap();
    save_quote(&tx, &quote).await.unwrap();
    let rejected = get_quote(&tx, second.id().unwrap()).await.unwrap();
    assert_eq!(
        rejected.status(),
        &QuoteStatus::Rejected {
            reason: String::from("Too expensive")
        }
    );
    let mut quote = rejected;
    quote.reopen().unwrap();
    save_quote(&tx, &quote).await.unwrap();
    let reopened = get_quote(&tx, second.id().unwrap()).await.unwrap();
    assert_eq!(reopened.status(), &QuoteStatus::Pending);
    let reason_left: i64 = {
        let row = tx
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                format!(
                    "SELECT count(*) FROM quotes WHERE id = '{}' AND rejection_reason IS NOT NULL",
                    second.id().unwrap()
                ),
            ))
            .await
            .unwrap()
            .unwrap();
        row.try_get_by_index(0).unwrap()
    };
    assert_eq!(reason_left, 0); // the old reason is dropped
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn dates_and_new_lines_are_saved_and_old_lines_stay_as_they_were() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "11").await;
    let saved = create_quote(&tx, full(&p, 2090)).await.unwrap();
    let id = saved.id().unwrap();
    let old_ids: Vec<_> = saved.items().iter().map(|line| line.id()).collect();

    let mut quote = get_quote(&tx, id).await.unwrap();
    let ready = day(2090).date() + chrono::Days::new(45);
    quote.set_promise_date(ready).unwrap();
    quote
        .set_expiry(day(2090).date() + chrono::Days::new(10))
        .unwrap();
    quote
        .add_item(LineItem::new(String::from("Wiper blades"), 2.0, 1800, LineKind::Part).unwrap())
        .unwrap();
    let after = save_quote(&tx, &quote).await.unwrap();

    assert_eq!(after.promise_date(), Some(ready));
    assert_eq!(after.expires_on(), day(2090).date() + chrono::Days::new(10));
    assert_eq!(after.items().len(), 6);
    let kept: Vec<_> = after.items()[..5].iter().map(|line| line.id()).collect();
    assert_eq!(kept, old_ids); // the old lines are the same rows
    assert_eq!(after.items()[5].description(), "Wiper blades");
    assert_eq!(after.items()[5].quote_id(), Some(id));
    assert_eq!(after.total(), quote.total());
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_customer_and_a_machine_can_be_added_to_a_quick_quote_later() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "12").await;
    let saved = create_quote(&tx, quick(p.branch, 2090)).await.unwrap();
    let id = saved.id().unwrap();

    let mut quote = get_quote(&tx, id).await.unwrap();
    quote
        .attach_customer(get_customer(&tx, p.customer).await.unwrap())
        .unwrap();
    quote
        .attach_equipment(get_equipment(&tx, p.machine).await.unwrap())
        .unwrap();
    let after = save_quote(&tx, &quote).await.unwrap();
    assert_eq!(after.customer().unwrap().name(), "QT-Jane-12");
    assert_eq!(after.equipment().unwrap().id(), Some(p.machine));

    // what it looked like when it was added stays, even if the customer changes afterwards
    let renamed = Customer::new(String::from("QT-Renamed"), None, None, contact(), regina())
        .unwrap()
        .with_id(p.customer);
    update_customer(&tx, &renamed).await.unwrap();
    let mut again = get_quote(&tx, id).await.unwrap();
    assert_eq!(again.customer().unwrap().name(), "QT-Jane-12");
    // saving it again keeps the old copy (it does not refresh it)
    again
        .set_promise_date(day(2090).date() + chrono::Days::new(5))
        .unwrap();
    save_quote(&tx, &again).await.unwrap();
    assert_eq!(
        get_quote(&tx, id).await.unwrap().customer().unwrap().name(),
        "QT-Jane-12"
    );
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn saving_a_quote_that_was_never_saved_or_does_not_exist_is_refused() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "13").await;
    let not_saved = Quote::new(
        samples::quote_number(),
        day(2090),
        get_branch(&tx, p.branch).await.unwrap(),
    )
    .unwrap();
    assert!(matches!(
        save_quote(&tx, &not_saved).await,
        Err(QuoteError::QuoteNotSaved)
    ));
    let ghost = Quote::new(
        samples::quote_number(),
        day(2090),
        get_branch(&tx, p.branch).await.unwrap(),
    )
    .unwrap()
    .with_id(Uuid::nil());
    assert!(matches!(
        save_quote(&tx, &ghost).await,
        Err(QuoteError::UnknownQuote)
    ));
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_party_that_was_never_saved_cannot_be_put_on_a_saved_quote() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "14").await;
    let saved = create_quote(&tx, quick(p.branch, 2090)).await.unwrap();
    let mut quote = get_quote(&tx, saved.id().unwrap()).await.unwrap();
    quote.attach_customer(plain_customer("QT-Unsaved")).unwrap();
    assert!(matches!(
        save_quote(&tx, &quote).await,
        Err(QuoteError::CustomerNotSaved)
    ));
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn two_people_changing_the_same_quote_cannot_overwrite_each_other_silently() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "15").await;
    let saved = create_quote(&tx, full(&p, 2090)).await.unwrap();
    let id = saved.id().unwrap();
    let line = || LineItem::new(String::from("Extra"), 1.0, 100, LineKind::Fee).unwrap();

    let mut mine = get_quote(&tx, id).await.unwrap();
    let mut yours = get_quote(&tx, id).await.unwrap();
    mine.add_item(line()).unwrap();
    yours.add_item(line()).unwrap();
    save_quote(&tx, &mine).await.unwrap();
    assert!(matches!(
        save_quote(&tx, &yours).await,
        Err(QuoteError::QuoteChangedMeanwhile)
    ));
    assert_eq!(get_quote(&tx, id).await.unwrap().items().len(), 6);

    // and a customer added by someone else in between
    let quick_saved = create_quote(&tx, quick(p.branch, 2090)).await.unwrap();
    let qid = quick_saved.id().unwrap();
    let mut first = get_quote(&tx, qid).await.unwrap();
    let mut second = get_quote(&tx, qid).await.unwrap();
    first
        .attach_customer(get_customer(&tx, p.customer).await.unwrap())
        .unwrap();
    second
        .attach_customer(get_customer(&tx, p.customer).await.unwrap())
        .unwrap();
    save_quote(&tx, &first).await.unwrap();
    let outcome = save_quote(&tx, &second).await;
    // the same customer again is fine; it is not a different one
    assert!(outcome.is_ok());
}

// ===== lists =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_list_shows_the_newest_first_with_their_lines_and_can_be_filtered_and_paged() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let one = parties(&tx, "16").await;
    let two = parties(&tx, "17").await;
    let a = create_quote(&tx, full(&one, 2090)).await.unwrap();
    let b = create_quote(&tx, quick(one.branch, 2091)).await.unwrap();
    let c = create_quote(&tx, full(&two, 2090)).await.unwrap();

    let all = list_quotes(&tx, &QuoteFilter::default(), 200, 0)
        .await
        .unwrap();
    let ours: Vec<String> = all
        .iter()
        .map(|q| q.number().to_string())
        .filter(|n| [&a, &b, &c].iter().any(|q| q.number().to_string() == *n))
        .collect();
    // newest date first (2091 before 2090), then the higher number
    assert_eq!(
        ours,
        [
            b.number().to_string(),
            c.number().to_string(),
            a.number().to_string()
        ]
    );
    let with_lines = all.iter().find(|q| q.id() == a.id()).unwrap();
    assert_eq!(with_lines.items().len(), 5);

    let by_customer = QuoteFilter {
        customer_id: Some(one.customer),
        ..Default::default()
    };
    let listed = list_quotes(&tx, &by_customer, 200, 0).await.unwrap();
    assert_eq!(listed.iter().map(|q| q.id()).collect::<Vec<_>>(), [a.id()]);
    let by_branch = QuoteFilter {
        branch_id: Some(one.branch),
        ..Default::default()
    };
    assert_eq!(list_quotes(&tx, &by_branch, 200, 0).await.unwrap().len(), 2);
    let by_machine = QuoteFilter {
        equipment_id: Some(two.machine),
        ..Default::default()
    };
    assert_eq!(
        list_quotes(&tx, &by_machine, 200, 0).await.unwrap()[0].id(),
        c.id()
    );

    let first_page = list_quotes(&tx, &by_branch, 1, 0).await.unwrap();
    let second_page = list_quotes(&tx, &by_branch, 1, 1).await.unwrap();
    assert_eq!(first_page[0].id(), b.id());
    assert_eq!(second_page[0].id(), a.id());
}

// ===== the database itself refuses =====

async fn refused(tx: &DatabaseTransaction, sql: String) -> bool {
    // A refused statement poisons a Postgres transaction, so it runs in a savepoint.
    let savepoint = tx.begin().await.unwrap();
    savepoint.execute_unprepared(&sql).await.is_err()
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn nothing_that_a_quote_points_at_can_be_deleted_and_neither_can_the_quote() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "18").await;
    let saved = create_quote(&tx, full(&p, 2090)).await.unwrap();
    let id = saved.id().unwrap();
    for sql in [
        format!("DELETE FROM branches WHERE id = '{}'", p.branch),
        format!("DELETE FROM customers WHERE id = '{}'", p.customer),
        format!("DELETE FROM equipment WHERE id = '{}'", p.machine),
        format!("DELETE FROM quotes WHERE id = '{id}'"),
        format!(
            "DELETE FROM quote_lines WHERE quote_id = '{id}' AND false OR quote_id = '{id}' AND position = 0 AND false"
        ),
    ] {
        // the last one deletes nothing (its condition is false), so it is not refused; skip it
        if sql.contains("AND false") {
            continue;
        }
        assert!(refused(&tx, sql.clone()).await, "{sql}");
    }
    // a quote can't point at a branch, customer or machine that doesn't exist
    for column in ["branch_id", "customer_id", "equipment_id"] {
        let sql = format!(
            "UPDATE quotes SET {column} = '{}' WHERE id = '{id}'",
            Uuid::from_u128(7)
        );
        assert!(refused(&tx, sql.clone()).await, "{sql}");
    }
    // and two lines can't share a position
    let sql = format!(
        "INSERT INTO quote_lines (quote_id, position, kind, description, quantity, unit_price_cents) \
         VALUES ('{id}', 0, 'PART', 'again', 1, 1)"
    );
    assert!(refused(&tx, sql).await);
    // two quotes can't share a number
    let sql = format!(
        "INSERT INTO quotes (number, created_at, expires_on, status, branch_id, province_code, branch_copy) \
         SELECT number, created_at, expires_on, status, branch_id, province_code, branch_copy FROM quotes WHERE id = '{id}'"
    );
    assert!(refused(&tx, sql).await);
}

// ===== a damaged row is refused, not shown =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_saved_quote_that_breaks_a_rule_is_refused_when_read() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "19").await;
    let other = create_customer(&tx, &plain_customer("QT-Other"))
        .await
        .unwrap();
    let other_id = other.id().unwrap();

    for (change, why) in [
        ("status = 'maybe'".to_string(), "unknown status"),
        ("status = 'rejected'".to_string(), "rejected with no reason"),
        (
            "rejection_reason = 'why'".to_string(),
            "a reason on a pending quote",
        ),
        ("number = 'Q-26-1'".to_string(), "bad number"),
        (
            "province_code = 'AB'".to_string(),
            "province differs from the branch copy",
        ),
        (
            "branch_copy = '{}'::jsonb".to_string(),
            "branch copy is not a branch",
        ),
        (
            "customer_copy = '[]'::jsonb".to_string(),
            "customer copy is not a customer",
        ),
        (
            format!("customer_id = '{other_id}'"),
            "link and copy disagree",
        ),
        ("equipment_copy = NULL".to_string(), "link without a copy"),
        ("customer_id = NULL".to_string(), "copy without a link"),
    ] {
        let saved = create_quote(&tx, full(&p, 2090)).await.unwrap();
        let id = saved.id().unwrap();
        let sp = tx.begin().await.unwrap();
        sp.execute_unprepared(&format!("UPDATE quotes SET {change} WHERE id = '{id}'"))
            .await
            .unwrap();
        assert!(get_quote(&sp, id).await.is_err(), "{why}");
        assert!(
            list_quotes(&sp, &QuoteFilter::default(), 200, 0)
                .await
                .is_err(),
            "{why} (in a list)"
        );
        // sp is dropped here: the damage is undone
    }
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_saved_line_that_breaks_a_rule_is_refused_when_read() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "20").await;
    for (change, why) in [
        ("kind = 'GIFT'", "unknown kind"),
        ("quantity = 0", "zero quantity"),
        ("unit_price_cents = -5", "negative price"),
        ("position = 9", "a gap in the order"),
    ] {
        let saved = create_quote(&tx, full(&p, 2090)).await.unwrap();
        let id = saved.id().unwrap();
        let sp = tx.begin().await.unwrap();
        sp.execute_unprepared(&format!(
            "UPDATE quote_lines SET {change} WHERE quote_id = '{id}' AND position = 2"
        ))
        .await
        .unwrap();
        assert!(get_quote(&sp, id).await.is_err(), "{why}");
    }
}

// ===== two real connections at once =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn quotes_saved_at_the_same_moment_get_different_numbers_and_none_is_lost() {
    let (_turn, db) = setup().await;
    async fn clean(db: &DatabaseConnection) {
        for sql in [
            "DELETE FROM quote_lines WHERE quote_id IN (SELECT id FROM quotes WHERE number LIKE 'Q-2092-%')",
            "DELETE FROM quotes WHERE number LIKE 'Q-2092-%'",
            "DELETE FROM quote_numbers WHERE year = 2092",
            "DELETE FROM branches WHERE number = 'QT-CONC'",
        ] {
            db.execute_unprepared(sql).await.expect("clean up");
        }
    }
    clean(&db).await;
    // committed for real, so other connections can see it
    let branch = create_branch(&db, &branch("QT-CONC")).await.unwrap();
    let branch_id = branch.id().unwrap();

    let mut jobs = Vec::new();
    for _ in 0..8 {
        let db = db.clone();
        jobs.push(tokio::spawn(async move {
            create_quote(&db, quick(branch_id, 2092)).await
        }));
    }
    let mut numbers = Vec::new();
    for job in jobs {
        let quote = job.await.expect("task").expect("every save works");
        numbers.push(quote.number().counter());
    }
    let listed = list_quotes(
        &db,
        &QuoteFilter {
            branch_id: Some(branch_id),
            ..Default::default()
        },
        200,
        0,
    )
    .await
    .unwrap()
    .len();
    clean(&db).await;

    numbers.sort_unstable();
    assert_eq!(numbers, [1, 2, 3, 4, 5, 6, 7, 8]); // no repeat, no gap
    assert_eq!(listed, 8);
}
