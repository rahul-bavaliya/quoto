// tests/database_repair_orders.rs: saving and loading repair orders in a real database
//
// These need a running database, so they are skipped normally (and in CI). Run them by hand:
//
//     docker compose up -d
//     cargo test --test database_repair_orders -- --ignored
//
// Almost every test works inside a transaction that is dropped at the end, which undoes it, so
// nothing is left behind. The tests that need several real connections at once commit, and remove
// their own rows. All people, machines and branches here are made up. Repair orders are dated in
// the years 2092, 2093, 2095 (undone at the end), 2094 and 2096 (committed, then removed), so their counters never touch a real year's.

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

fn later(year: i32, hour: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(year, 3, 15)
        .and_then(|day| day.and_hms_opt(hour, 0, 0))
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

fn branch(number: &str) -> Branch {
    Branch::new(
        number.to_string(),
        String::from("QT Regina Branch"),
        regina(),
        PhoneNumber::parse("306-555-0100").unwrap(),
    )
    .unwrap()
}

/// A tractor; `reading` None means no meter reading on the quote.
fn machine(serial: &str, reading: Option<f64>) -> Equipment {
    Equipment::new(
        String::from("John Deere"),
        String::from("8R 310"),
        Some(2021),
        Identification::new(Some(serial.to_string()), Some(String::from("QT-1234"))).unwrap(),
        Meter::new(reading, MeterType::Hour).unwrap(),
        EquipmentType::Tractor,
    )
    .unwrap()
}

fn plain_customer(name: &str) -> Customer {
    Customer::new(
        name.to_string(),
        Some(String::from("Prairie Farms Ltd.")),
        None,
        Contact::new(PhoneNumber::parse("306-555-0142").unwrap(), None, None),
        regina(),
    )
    .unwrap()
}

struct Parties {
    branch: Uuid,
    customer: Uuid,
    machine: Uuid,
}

async fn parties(db: &impl ConnectionTrait, tag: &str, reading: Option<f64>) -> Parties {
    let branch = create_branch(db, &branch(&format!("QT-{tag}")))
        .await
        .unwrap();
    let customer = create_customer(db, &plain_customer(&format!("QT-Jane-{tag}")))
        .await
        .unwrap();
    let machine = create_equipment(
        db,
        &machine(&format!("QT-RO-SERIAL-{tag}"), reading).with_customer_id(customer.id().unwrap()),
    )
    .await
    .unwrap();
    Parties {
        branch: branch.id().unwrap(),
        customer: customer.id().unwrap(),
        machine: machine.id().unwrap(),
    }
}

/// A quote with the standard oil change; `accept` decides whether the customer said yes.
async fn quote_for<C: ConnectionTrait + TransactionTrait>(
    db: &C,
    p: &Parties,
    year: i32,
    accept: bool,
) -> Quote {
    let quote = create_quote(
        db,
        NewQuote {
            created_at: day(year),
            branch_id: p.branch,
            customer_id: Some(p.customer),
            equipment_id: Some(p.machine),
            items: samples::oil_change_lines().unwrap(),
        },
    )
    .await
    .unwrap();
    if !accept {
        return quote;
    }
    let mut quote = quote;
    quote.accept(day(year).date()).unwrap();
    save_quote(db, &quote).await.unwrap()
}

fn open_from(quote: &Quote, year: i32, reading: Option<f64>) -> NewRepairOrder {
    NewRepairOrder {
        created_at: day(year),
        quote_id: quote.id().unwrap(),
        meter_reading: reading,
    }
}

// ===== create and read =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn an_accepted_quote_becomes_a_saved_open_repair_order_with_a_number_and_its_quote() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "R1", Some(1250.5)).await;
    let quote = quote_for(&tx, &p, 2092, true).await;

    let order = create_repair_order(&tx, open_from(&quote, 2092, None))
        .await
        .unwrap();
    assert_eq!(order.number().to_string(), "RO-2092-00000001");
    assert_eq!(order.status(), &RepairOrderStatus::Open);
    assert_eq!(order.version(), 0);
    assert!(order.id().is_some()); // the database made it
    assert_eq!(order.created_at(), day(2092));
    assert_eq!(
        (order.meter_reading(), order.meter_type()),
        (1250.5, MeterType::Hour)
    );
    assert_eq!((order.started_at(), order.closed_at()), (None, None));
    // the work is the accepted quote's, read from the quote
    assert_eq!(order.quote().id(), quote.id());
    assert_eq!(order.quote().total(), 21978);
    assert_eq!(order.quote().items().len(), 5);
    assert_eq!(order.quote().customer().unwrap().id(), Some(p.customer));
    assert_eq!(order.quote().equipment().unwrap().id(), Some(p.machine));

    // reading it back gives the same thing
    let again = get_repair_order(&tx, order.id().unwrap()).await.unwrap();
    assert_eq!(again.number().to_string(), "RO-2092-00000001");
    assert_eq!(again.quote().total(), 21978);
    assert_eq!(again.meter_reading(), 1250.5);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_meter_reading_the_writer_gives_wins_and_a_missing_one_is_refused_and_leaves_nothing() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let with = parties(&tx, "R2", Some(1250.5)).await;
    let without = parties(&tx, "R3", None).await;
    let quote_with = quote_for(&tx, &with, 2092, true).await;
    let quote_without = quote_for(&tx, &without, 2092, true).await;

    // no reading anywhere: refused, and the number is given back
    let error = create_repair_order(&tx, open_from(&quote_without, 2092, None))
        .await
        .map(|_| ())
        .unwrap_err();
    assert_eq!(error.code(), "METER_READING_REQUIRED");
    let bad = create_repair_order(&tx, open_from(&quote_without, 2092, Some(-1.0)))
        .await
        .map(|_| ())
        .unwrap_err();
    assert_eq!(bad.code(), "INVALID_METER_READING");
    // giving it works, and it is the first number of the year (the refused ones took none)
    let order = create_repair_order(&tx, open_from(&quote_without, 2092, Some(1500.0)))
        .await
        .unwrap();
    assert_eq!(order.number().to_string(), "RO-2092-00000001");
    assert_eq!(order.meter_reading(), 1500.0);
    // today's reading beats the one on the quote
    let second = create_repair_order(&tx, open_from(&quote_with, 2092, Some(1262.0)))
        .await
        .unwrap();
    assert_eq!(second.meter_reading(), 1262.0);
    assert_eq!(second.number().to_string(), "RO-2092-00000002");
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn only_an_accepted_quote_that_exists_can_become_a_repair_order() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "R4", Some(10.0)).await;
    let pending = quote_for(&tx, &p, 2092, false).await;
    let mut rejected = quote_for(&tx, &p, 2092, false).await;
    rejected.reject("too dear").unwrap();
    let rejected = save_quote(&tx, &rejected).await.unwrap();
    let quick = create_quote(
        &tx,
        NewQuote {
            created_at: day(2092),
            branch_id: p.branch,
            customer_id: None,
            equipment_id: None,
            items: Vec::new(),
        },
    )
    .await
    .unwrap();
    for quote in [&pending, &rejected, &quick] {
        let error = create_repair_order(&tx, open_from(quote, 2092, None))
            .await
            .map(|_| ())
            .unwrap_err();
        assert_eq!(error.code(), "QUOTE_NOT_ACCEPTED");
    }
    let nobody = NewRepairOrder {
        created_at: day(2092),
        quote_id: Uuid::from_u128(0xDEAD),
        meter_reading: Some(1.0),
    };
    let error = create_repair_order(&tx, nobody)
        .await
        .map(|_| ())
        .unwrap_err();
    assert_eq!(error.code(), "UNKNOWN_QUOTE");
    // none of those took a number
    let ok = quote_for(&tx, &p, 2092, true).await;
    let order = create_repair_order(&tx, open_from(&ok, 2092, None))
        .await
        .unwrap();
    assert_eq!(order.number().to_string(), "RO-2092-00000001");
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn one_quote_makes_at_most_one_repair_order() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "R5", Some(10.0)).await;
    let quote = quote_for(&tx, &p, 2092, true).await;
    create_repair_order(&tx, open_from(&quote, 2092, None))
        .await
        .unwrap();
    let error = create_repair_order(&tx, open_from(&quote, 2092, None))
        .await
        .map(|_| ())
        .unwrap_err();
    assert_eq!(error.code(), "REPAIR_ORDER_EXISTS");
    assert_eq!(error.kind(), ErrorKind::BusinessRule);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn numbers_count_up_inside_a_year_and_restart_in_the_next() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "R6", Some(10.0)).await;
    let mut numbers = Vec::new();
    for year in [2092, 2092, 2092, 2095, 2092, 2095] {
        let quote = quote_for(&tx, &p, year, true).await;
        let order = create_repair_order(&tx, open_from(&quote, year, None))
            .await
            .unwrap();
        numbers.push(order.number().to_string());
    }
    assert_eq!(
        numbers,
        [
            "RO-2092-00000001",
            "RO-2092-00000002",
            "RO-2092-00000003",
            "RO-2095-00000001",
            "RO-2092-00000004",
            "RO-2095-00000002"
        ]
    );
}

// ===== the flow, saved =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn every_saved_change_adds_one_to_the_version_and_a_stale_change_is_refused() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "R7", Some(10.0)).await;
    let quote = quote_for(&tx, &p, 2092, true).await;
    let order = create_repair_order(&tx, open_from(&quote, 2092, None))
        .await
        .unwrap();
    let id = order.id().unwrap();

    // someone else reads it too, at version 0
    let mut other = get_repair_order(&tx, id).await.unwrap();

    let mut mine = order;
    mine.start(later(2092, 8)).unwrap();
    let mine = save_repair_order(&tx, &mine).await.unwrap();
    assert_eq!(mine.version(), 1);
    assert_eq!(mine.status(), &RepairOrderStatus::InProgress);
    assert_eq!(mine.started_at(), Some(later(2092, 8)));

    // the other one is still holding version 0: refused, nothing changes
    other.cancel("changed my mind", later(2092, 9)).unwrap();
    let error = save_repair_order(&tx, &other)
        .await
        .map(|_| ())
        .unwrap_err();
    assert_eq!(error.code(), "REPAIR_ORDER_CHANGED_MEANWHILE");
    let now = get_repair_order(&tx, id).await.unwrap();
    assert_eq!((now.version(), now.status().label()), (1, "in_progress"));

    // read again and redo: it goes through
    let mut again = get_repair_order(&tx, id).await.unwrap();
    again.complete(later(2092, 16)).unwrap();
    let done = save_repair_order(&tx, &again).await.unwrap();
    assert_eq!((done.version(), done.status().label()), (2, "completed"));
    assert_eq!(done.closed_at(), Some(later(2092, 16)));
    // a change that does not move anything still counts as a change (a person pressed save)
    let mut same = get_repair_order(&tx, id).await.unwrap();
    same.set_due_date(None).unwrap_err(); // closed: the library already refuses
    assert_eq!(get_repair_order(&tx, id).await.unwrap().version(), 2);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_cancellation_and_a_due_date_are_saved_and_read_back() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "R8", Some(10.0)).await;
    let quote = quote_for(&tx, &p, 2092, true).await;
    let mut order = create_repair_order(&tx, open_from(&quote, 2092, None))
        .await
        .unwrap();
    let id = order.id().unwrap();

    order
        .set_due_date(Some(NaiveDate::from_ymd_opt(2092, 4, 1).unwrap()))
        .unwrap();
    let order = save_repair_order(&tx, &order).await.unwrap();
    assert_eq!(order.due_date(), NaiveDate::from_ymd_opt(2092, 4, 1));
    let mut order = get_repair_order(&tx, id).await.unwrap();
    order.set_due_date(None).unwrap();
    let order = save_repair_order(&tx, &order).await.unwrap();
    assert_eq!(order.due_date(), None);

    let mut order = order;
    order
        .cancel("  part is discontinued  ", later(2092, 10))
        .unwrap();
    let saved = save_repair_order(&tx, &order).await.unwrap();
    let read = get_repair_order(&tx, id).await.unwrap();
    for each in [&saved, &read] {
        assert_eq!(
            each.status(),
            &RepairOrderStatus::Cancelled {
                reason: String::from("part is discontinued")
            }
        );
        assert_eq!(each.closed_at(), Some(later(2092, 10)));
        assert_eq!(each.version(), 3);
    }
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_repair_order_that_was_never_saved_or_is_unknown_can_not_be_updated() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let p = parties(&tx, "R9", Some(10.0)).await;
    let quote = quote_for(&tx, &p, 2092, true).await;
    let order = create_repair_order(&tx, open_from(&quote, 2092, None))
        .await
        .unwrap();
    // an object built in memory has no id
    let fresh = RepairOrder::open(
        RepairOrderNumber::new(2092, 99).unwrap(),
        day(2092),
        get_quote(&tx, quote.id().unwrap()).await.unwrap(),
        None,
    )
    .unwrap();
    let error = save_repair_order(&tx, &fresh)
        .await
        .map(|_| ())
        .unwrap_err();
    assert_eq!(error.code(), "REPAIR_ORDER_NOT_SAVED");
    let error = get_repair_order(&tx, Uuid::from_u128(0xBEEF))
        .await
        .map(|_| ())
        .unwrap_err();
    assert_eq!(error.code(), "UNKNOWN_REPAIR_ORDER");
    assert!(order.id().is_some());
}

// ===== lists =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_list_shows_the_newest_first_can_be_filtered_by_everything_and_paged() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let one = parties(&tx, "R10", Some(1.0)).await;
    let two = parties(&tx, "R11", Some(2.0)).await;
    let mut made = Vec::new();
    for (p, year) in [(&one, 2092), (&one, 2093), (&two, 2092)] {
        let quote = quote_for(&tx, p, year, true).await;
        made.push(
            create_repair_order(&tx, open_from(&quote, year, None))
                .await
                .unwrap(),
        );
    }
    let (a, b, c) = (&made[0], &made[1], &made[2]);
    let mut started = get_repair_order(&tx, c.id().unwrap()).await.unwrap();
    started.start(later(2092, 8)).unwrap();
    save_repair_order(&tx, &started).await.unwrap();

    let ours = |orders: &[RepairOrder]| -> Vec<String> {
        orders
            .iter()
            .map(|o| o.number().to_string())
            .filter(|n| made.iter().any(|m| m.number().to_string() == *n))
            .collect()
    };
    let all = list_repair_orders(&tx, &RepairOrderFilter::default(), 200, 0)
        .await
        .unwrap();
    // 2093 before 2092, then the higher number
    assert_eq!(
        ours(&all),
        [
            b.number().to_string(),
            c.number().to_string(),
            a.number().to_string()
        ]
    );
    // every row of the page came back with its quote
    assert!(all.iter().all(|o| o.quote().total() > 0));

    let by = |filter: RepairOrderFilter| {
        let tx = &tx;
        async move { ours(&list_repair_orders(tx, &filter, 200, 0).await.unwrap()) }
    };
    let mine = by(RepairOrderFilter {
        customer_id: Some(one.customer),
        ..Default::default()
    })
    .await;
    assert_eq!(mine, [b.number().to_string(), a.number().to_string()]);
    let at_branch = by(RepairOrderFilter {
        branch_id: Some(two.branch),
        ..Default::default()
    })
    .await;
    assert_eq!(at_branch, [c.number().to_string()]);
    let for_machine = by(RepairOrderFilter {
        equipment_id: Some(one.machine),
        ..Default::default()
    })
    .await;
    assert_eq!(
        for_machine,
        [b.number().to_string(), a.number().to_string()]
    );
    let in_progress = by(RepairOrderFilter {
        status: Some("in_progress"),
        ..Default::default()
    })
    .await;
    assert_eq!(in_progress, [c.number().to_string()]);
    let none = by(RepairOrderFilter {
        status: Some("completed"),
        ..Default::default()
    })
    .await;
    assert!(none.is_empty());

    // paging: one at a time, in the same order
    let filter = RepairOrderFilter {
        customer_id: Some(one.customer),
        ..Default::default()
    };
    let first = list_repair_orders(&tx, &filter, 1, 0).await.unwrap();
    let second = list_repair_orders(&tx, &filter, 1, 1).await.unwrap();
    let third = list_repair_orders(&tx, &filter, 1, 2).await.unwrap();
    assert_eq!(first[0].number().to_string(), b.number().to_string());
    assert_eq!(second[0].number().to_string(), a.number().to_string());
    assert!(third.is_empty());
}

// ===== the database's own rules =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_database_refuses_to_delete_what_a_repair_order_points_at() {
    let (_turn, db) = setup().await;
    clean_2096(&db).await; // in case an earlier run stopped half way
    let p = parties(&db, "R12", Some(10.0)).await;
    // Committed for real: a refused DELETE inside a transaction would poison it.
    let quote = quote_for(&db, &p, 2096, true).await;
    let order = create_repair_order(&db, open_from(&quote, 2096, None))
        .await
        .unwrap();
    let id = order.id().unwrap();
    let quote_id = quote.id().unwrap();
    let mut refused = Vec::new();
    for sql in [
        format!("DELETE FROM quotes WHERE id = '{quote_id}'"),
        format!("DELETE FROM branches WHERE id = '{}'", p.branch),
        format!("DELETE FROM customers WHERE id = '{}'", p.customer),
        format!("DELETE FROM equipment WHERE id = '{}'", p.machine),
    ] {
        refused.push(db.execute_unprepared(&sql).await.is_err());
    }
    // the same quote can't have two repair orders, even by hand
    let twice = db
        .execute_unprepared(&format!(
            "INSERT INTO repair_orders (number, quote_id, branch_id, customer_id, equipment_id, \
             created_at, status, meter_reading, meter_type) SELECT 'RO-2096-77777777', quote_id, \
             branch_id, customer_id, equipment_id, created_at, status, meter_reading, meter_type \
             FROM repair_orders WHERE id = '{id}'"
        ))
        .await
        .is_err();
    clean_2096(&db).await;
    assert_eq!(refused, [true, true, true, true]);
    assert!(twice);
}

async fn clean_2096(db: &DatabaseConnection) {
    for sql in [
        "DELETE FROM repair_orders WHERE number LIKE 'RO-2096-%'",
        "DELETE FROM repair_order_numbers WHERE year = 2096",
        "DELETE FROM quote_lines WHERE quote_id IN (SELECT id FROM quotes WHERE number LIKE 'Q-2096-%')",
        "DELETE FROM quotes WHERE number LIKE 'Q-2096-%'",
        "DELETE FROM quote_numbers WHERE year = 2096",
        "DELETE FROM equipment WHERE serial_number = 'QT-RO-SERIAL-R12'",
        "DELETE FROM customers WHERE name = 'QT-Jane-R12'",
        "DELETE FROM branches WHERE number = 'QT-R12'",
    ] {
        db.execute_unprepared(sql).await.expect("clean up");
    }
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_row_damaged_behind_quotos_back_is_refused_not_shown() {
    let (_turn, db) = setup().await;
    clean_2096(&db).await; // in case an earlier run stopped half way
    let p = parties(&db, "R12", Some(10.0)).await;
    let quote = quote_for(&db, &p, 2096, true).await;
    let order = create_repair_order(&db, open_from(&quote, 2096, None))
        .await
        .unwrap();
    let id = order.id().unwrap();
    let mut results = Vec::new();
    for sql in [
        "UPDATE repair_orders SET status = 'bogus'",
        "UPDATE repair_orders SET status = 'open', cancellation_reason = 'why'",
        "UPDATE repair_orders SET status = 'cancelled', cancellation_reason = NULL",
        "UPDATE repair_orders SET status = 'in_progress', cancellation_reason = NULL, started_at = NULL",
        "UPDATE repair_orders SET status = 'completed', started_at = created_at, closed_at = NULL",
        "UPDATE repair_orders SET status = 'open', started_at = created_at, closed_at = NULL",
        "UPDATE repair_orders SET status = 'open', started_at = NULL, meter_reading = -5",
        "UPDATE repair_orders SET meter_reading = 5, meter_type = 'parsecs'",
        "UPDATE repair_orders SET meter_type = 'hr', number = 'RO-2096-ZZ'",
    ] {
        // only our own row, never anyone else's
        db.execute_unprepared(&format!("{sql} WHERE id = '{id}'"))
            .await
            .expect("damage the row");
        results.push(
            get_repair_order(&db, id)
                .await
                .map(|_| ())
                .unwrap_err()
                .code(),
        );
        // put it back for the next one
        db.execute_unprepared(&format!(
            "UPDATE repair_orders SET status = 'open', cancellation_reason = NULL, \
             started_at = NULL, closed_at = NULL, meter_reading = 10, meter_type = 'hr', \
             number = 'RO-2096-00000001', customer_id = '{}' WHERE id = '{id}'",
            p.customer
        ))
        .await
        .unwrap();
    }
    // after all that, the repaired row reads fine again
    let healthy = get_repair_order(&db, id).await.map(|o| o.status().label());
    clean_2096(&db).await;
    assert_eq!(results, ["STORED_REPAIR_ORDER_DAMAGED"; 9]);
    assert_eq!(healthy.unwrap(), "open");
}

// ===== two people at once =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn eight_people_opening_a_repair_order_from_one_quote_at_once_one_wins() {
    let (_turn, db) = setup().await;
    async fn clean(db: &DatabaseConnection) {
        for sql in [
            "DELETE FROM repair_orders WHERE number LIKE 'RO-2094-%'",
            "DELETE FROM repair_order_numbers WHERE year = 2094",
            "DELETE FROM quote_lines WHERE quote_id IN (SELECT id FROM quotes WHERE number LIKE 'Q-2094-%')",
            "DELETE FROM quotes WHERE number LIKE 'Q-2094-%'",
            "DELETE FROM quote_numbers WHERE year = 2094",
            "DELETE FROM equipment WHERE serial_number = 'QT-RO-SERIAL-R13'",
            "DELETE FROM customers WHERE name = 'QT-Jane-R13'",
            "DELETE FROM branches WHERE number = 'QT-R13'",
        ] {
            db.execute_unprepared(sql).await.expect("clean up");
        }
    }
    clean(&db).await;
    let p = parties(&db, "R13", Some(10.0)).await;
    let quote = quote_for(&db, &p, 2094, true).await;

    let mut jobs = Vec::new();
    for _ in 0..8 {
        let db = db.clone();
        let new = open_from(&quote, 2094, None);
        jobs.push(tokio::spawn(
            async move { create_repair_order(&db, new).await },
        ));
    }
    let (mut won, mut told) = (0, 0);
    for job in jobs {
        match job.await.unwrap() {
            Ok(_) => won += 1,
            Err(QuoteError::RepairOrderExists) => told += 1,
            Err(other) => panic!("unexpected: {other}"),
        }
    }
    let listed = list_repair_orders(
        &db,
        &RepairOrderFilter {
            customer_id: Some(p.customer),
            ..Default::default()
        },
        50,
        0,
    )
    .await
    .unwrap();

    // and then eight people read the repair order and try to start / cancel it at once
    let order = listed[0].id().unwrap();
    let mut readers = Vec::new();
    for _ in 0..8 {
        readers.push(get_repair_order(&db, order).await.unwrap());
    }
    let mut saves = Vec::new();
    for (n, mut each) in readers.into_iter().enumerate() {
        let db = db.clone();
        saves.push(tokio::spawn(async move {
            if n % 2 == 0 {
                each.start(later(2094, 8)).unwrap();
            } else {
                each.cancel("not this time", later(2094, 8)).unwrap();
            }
            save_repair_order(&db, &each).await
        }));
    }
    let (mut saved, mut changed) = (0, 0);
    for save in saves {
        match save.await.unwrap() {
            Ok(_) => saved += 1,
            Err(QuoteError::RepairOrderChangedMeanwhile) => changed += 1,
            Err(other) => panic!("unexpected: {other}"),
        }
    }
    let end = get_repair_order(&db, order).await.unwrap();
    clean(&db).await;
    assert_eq!((won, told), (1, 7));
    assert_eq!(listed.len(), 1);
    assert_eq!((saved, changed), (1, 7));
    assert_eq!(end.version(), 1);
    assert_ne!(end.status(), &RepairOrderStatus::Open);
}
