// tests/database_provinces.rs: the provinces table in a real database
//
// These need a running database, so they are skipped normally (and in CI). Run them by hand:
//
//     docker compose up -d
//     cargo test --test database_provinces -- --ignored
//
// They build the table and the 13 starting rows first (safe to repeat). Every change a test
// makes happens inside a transaction that is dropped at the end, which undoes it, so the
// database is left as it was found. (The rules about which tax mixes are allowed need no
// database: see tests/tax_rates.rs.)

use chrono::NaiveDate;
use quoto::*;
use sea_orm::{ConnectionTrait, DatabaseConnection, DatabaseTransaction, TransactionTrait};

/// The tests take turns: two of them building the table at the same moment would collide.
static ONE_AT_A_TIME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Waits for its turn, connects (settings from the environment or `.env`), builds the table
/// and the starting rows. Keep the first part alive for the whole test.
async fn setup() -> (tokio::sync::MutexGuard<'static, ()>, DatabaseConnection) {
    let turn = ONE_AT_A_TIME.lock().await;
    dotenvy::dotenv().ok();
    let settings = DatabaseSettings::from_env().expect("database settings (see .env.example)");
    let db = database::connect(&settings).await.expect("database");
    database::sync(&db).await.expect("sync");
    seed_provinces(&db).await.expect("seed");
    (turn, db)
}

/// A transaction that is never committed: dropping it undoes everything.
async fn work(db: &DatabaseConnection) -> DatabaseTransaction {
    db.begin().await.expect("transaction")
}

fn rate(thousandths: i32) -> TaxRate {
    TaxRate::from_thousandths(thousandths).unwrap()
}

fn day(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

fn gst_pst(gst: i32, pst: Option<i32>) -> TaxRates {
    TaxRates::GstPst {
        gst: rate(gst),
        pst: pst.map(rate),
    }
}

// ===== the starting rows =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn all_thirteen_provinces_and_territories_are_there() {
    let (_turn, db) = setup().await;
    let all = all_rates(&db).await.unwrap();
    assert_eq!(all.len(), 13);
    let codes: Vec<&str> = all.iter().map(|(code, _)| code.as_str()).collect();
    assert_eq!(
        codes,
        [
            "AB", "BC", "MB", "NB", "NL", "NS", "NT", "NU", "ON", "PE", "QC", "SK", "YT"
        ]
    );
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_seeded_rates_match_the_cra_table() {
    let (_turn, db) = setup().await;
    let check = |code: &'static str, expected: TaxRates| {
        let db = &db;
        async move { assert_eq!(rates_for(db, code).await.unwrap(), expected, "{code}") }
    };
    check("SK", gst_pst(5000, Some(6000))).await;
    check("AB", gst_pst(5000, None)).await;
    check("BC", gst_pst(5000, Some(7000))).await;
    check("MB", gst_pst(5000, Some(7000))).await;
    check("QC", gst_pst(5000, Some(9975))).await;
    check("ON", TaxRates::Hst(rate(13_000))).await;
    check("NS", TaxRates::Hst(rate(14_000))).await;
    for code in ["NB", "NL", "PE"] {
        check(code, TaxRates::Hst(rate(15_000))).await;
    }
    for code in ["YT", "NT", "NU"] {
        check(code, gst_pst(5000, None)).await;
    }
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn setting_up_again_is_safe_and_does_not_put_a_changed_rate_back() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    change_rates(
        &tx,
        "SK",
        &gst_pst(5000, Some(7000)),
        day(2027, 4, 1),
        "test",
    )
    .await
    .unwrap();

    seed_provinces(&tx).await.unwrap(); // a second seed in the same transaction
    assert_eq!(all_rates(&tx).await.unwrap().len(), 13);
    assert_eq!(
        rates_for(&tx, "SK").await.unwrap(),
        gst_pst(5000, Some(7000))
    );
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn an_unknown_province_is_refused() {
    let (_turn, db) = setup().await;
    assert_eq!(
        rates_for(&db, "ZZ").await,
        Err(QuoteError::UnknownProvince { code: "ZZ".into() })
    );
}

// ===== changing a rate (what the admin does) =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_rate_can_be_changed_in_one_update() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    change_rates(
        &tx,
        "SK",
        &gst_pst(5000, Some(7000)),
        day(2027, 4, 1),
        "test change",
    )
    .await
    .unwrap();
    assert_eq!(
        rates_for(&tx, "SK").await.unwrap(),
        gst_pst(5000, Some(7000))
    );
    // The other provinces are untouched.
    assert_eq!(rates_for(&tx, "AB").await.unwrap(), gst_pst(5000, None));
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_province_can_switch_from_gst_and_pst_to_hst() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    change_rates(
        &tx,
        "SK",
        &TaxRates::Hst(rate(11_000)),
        day(2027, 1, 1),
        "test switch",
    )
    .await
    .unwrap();
    assert_eq!(
        rates_for(&tx, "SK").await.unwrap(),
        TaxRates::Hst(rate(11_000))
    );
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_new_date_cannot_be_before_the_current_rates_date() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    // The starting rows are "in force since 2025-04-01".
    assert_eq!(
        change_rates(
            &tx,
            "SK",
            &gst_pst(5000, None),
            day(2024, 1, 1),
            "too early"
        )
        .await,
        Err(QuoteError::RatesDateBeforeCurrent {
            date: day(2024, 1, 1),
            current: day(2025, 4, 1)
        })
    );
    assert_eq!(
        rates_for(&tx, "SK").await.unwrap(),
        gst_pst(5000, Some(6000))
    );
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_blank_note_is_refused() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    assert_eq!(
        change_rates(&tx, "SK", &gst_pst(5000, None), day(2027, 1, 1), "   ").await,
        Err(QuoteError::MissingField { field: "note" })
    );
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn changing_an_unknown_province_is_refused() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    assert_eq!(
        change_rates(&tx, "ZZ", &gst_pst(5000, None), day(2027, 1, 1), "test").await,
        Err(QuoteError::UnknownProvince { code: "ZZ".into() })
    );
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn updated_at_moves_when_the_rates_change() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    change_rates(
        &tx,
        "SK",
        &gst_pst(5000, Some(7000)),
        day(2027, 4, 1),
        "test",
    )
    .await
    .unwrap();
    let row = tx
        .query_one_raw(sea_orm::Statement::from_string(
            sea_orm::DbBackend::Postgres,
            "SELECT updated_at > created_at AS moved FROM provinces WHERE code = 'SK'",
        ))
        .await
        .unwrap()
        .expect("a row");
    assert!(row.try_get::<bool>("", "moved").unwrap());
}

// ===== a damaged row is caught when it is read =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_row_with_a_gst_and_an_hst_is_refused_when_it_is_read() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    // Written behind Quoto's back (the table itself no longer has any rules).
    tx.execute_unprepared("UPDATE provinces SET hst_rate_thousandths = 13000 WHERE code = 'AB'")
        .await
        .unwrap();
    assert_eq!(
        rates_for(&tx, "AB").await,
        Err(QuoteError::InvalidTaxSystem)
    );
    assert_eq!(all_rates(&tx).await, Err(QuoteError::InvalidTaxSystem));
}
