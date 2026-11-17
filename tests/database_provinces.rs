// tests/database_provinces.rs: the provinces table and its rate history, in a real database
//
// These need a running database, so they are skipped normally (and in CI). Run them by hand:
//
//     docker compose up -d
//     cargo test --test database_provinces -- --ignored
//
// They build the tables first (safe to repeat). Every test works inside a transaction
// that is rolled back at the end, so the database is left as it was found.

use quoto::{DatabaseSettings, database};
use sqlx::{PgPool, Postgres, Transaction};

/// The tests take turns: `TRUNCATE` (tried in two of them) locks the whole table, and would
/// otherwise collide with a test running at the same moment.
static ONE_AT_A_TIME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Waits for its turn, connects (settings from the environment or `.env`) and makes sure
/// the tables exist. Keep the first part alive for the whole test.
async fn setup() -> (tokio::sync::MutexGuard<'static, ()>, PgPool) {
    let turn = ONE_AT_A_TIME.lock().await;
    dotenvy::dotenv().ok();
    let settings = DatabaseSettings::from_env().expect("database settings (see .env.example)");
    let pool = database::connect(&settings).await.expect("database");
    database::migrate(&pool).await.expect("migrations");
    (turn, pool)
}

/// A transaction that is never committed: dropping it undoes everything.
async fn work(pool: &PgPool) -> Transaction<'_, Postgres> {
    pool.begin().await.expect("transaction")
}

/// The five-character error code PostgreSQL gave (e.g. "23514" = a rule was broken).
fn sqlstate(error: sqlx::Error) -> String {
    error
        .as_database_error()
        .and_then(|e| e.code().map(|c| c.to_string()))
        .unwrap_or_else(|| format!("not a database error: {error}"))
}

const CHECK_VIOLATION: &str = "23514";
const UNIQUE_VIOLATION: &str = "23505";
const RESTRICT_VIOLATION: &str = "23001";

/// Runs one statement that must be refused, in its own transaction, and returns the code.
async fn refused(pool: &PgPool, sql: &str) -> String {
    let mut tx = work(pool).await;
    sqlstate(sqlx::query(sql).execute(&mut *tx).await.unwrap_err())
}

async fn count(tx: &mut Transaction<'_, Postgres>, sql: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(sql)
        .fetch_one(&mut **tx)
        .await
        .unwrap()
}

/// (gst, pst, hst) of one province.
async fn rates(
    tx: &mut Transaction<'_, Postgres>,
    code: &str,
) -> (Option<i32>, Option<i32>, Option<i32>) {
    sqlx::query_as("SELECT gst_rate_thousandths, pst_rate_thousandths, hst_rate_thousandths FROM provinces WHERE code = $1")
        .bind(code)
        .fetch_one(&mut **tx)
        .await
        .unwrap()
}

// ===== the seed =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn all_thirteen_provinces_and_territories_are_there() {
    let (_turn, pool) = setup().await;
    let mut tx = work(&pool).await;
    assert_eq!(count(&mut tx, "SELECT count(*) FROM provinces").await, 13);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_seeded_rates_match_the_cra_table() {
    let (_turn, pool) = setup().await;
    let mut tx = work(&pool).await;
    assert_eq!(rates(&mut tx, "SK").await, (Some(5000), Some(6000), None));
    assert_eq!(rates(&mut tx, "AB").await, (Some(5000), None, None));
    assert_eq!(rates(&mut tx, "BC").await, (Some(5000), Some(7000), None));
    assert_eq!(rates(&mut tx, "MB").await, (Some(5000), Some(7000), None));
    assert_eq!(rates(&mut tx, "QC").await, (Some(5000), Some(9975), None));
    assert_eq!(rates(&mut tx, "ON").await, (None, None, Some(13000)));
    assert_eq!(rates(&mut tx, "NS").await, (None, None, Some(14000)));
    for code in ["NB", "NL", "PE"] {
        assert_eq!(rates(&mut tx, code).await, (None, None, Some(15000)));
    }
    for code in ["YT", "NT", "NU"] {
        assert_eq!(rates(&mut tx, code).await, (Some(5000), None, None));
    }
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn running_the_migrations_again_is_safe() {
    let (_turn, pool) = setup().await;
    database::migrate(&pool).await.unwrap();
    database::migrate(&pool).await.unwrap();
}

// ===== a province must have its tax system =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_gst_and_an_hst_together_are_refused() {
    let (_turn, pool) = setup().await;
    let sql = "UPDATE provinces SET hst_rate_thousandths = 13000 WHERE code = 'AB'";
    assert_eq!(refused(&pool, sql).await, CHECK_VIOLATION);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_province_with_no_tax_at_all_is_refused() {
    let (_turn, pool) = setup().await;
    let sql = "UPDATE provinces SET gst_rate_thousandths = NULL WHERE code = 'AB'";
    assert_eq!(refused(&pool, sql).await, CHECK_VIOLATION);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn an_hst_province_cannot_also_have_a_pst() {
    let (_turn, pool) = setup().await;
    let sql = "UPDATE provinces SET pst_rate_thousandths = 1000 WHERE code = 'ON'";
    assert_eq!(refused(&pool, sql).await, CHECK_VIOLATION);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_rate_must_be_above_zero_and_at_most_a_hundred_percent() {
    let (_turn, pool) = setup().await;
    for value in ["0", "-1", "100001"] {
        let sql = format!("UPDATE provinces SET pst_rate_thousandths = {value} WHERE code = 'SK'");
        assert_eq!(refused(&pool, &sql).await, CHECK_VIOLATION, "{value}");
    }
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_blank_note_is_refused() {
    let (_turn, pool) = setup().await;
    let sql = "UPDATE provinces SET note = '   ' WHERE code = 'SK'";
    assert_eq!(refused(&pool, sql).await, CHECK_VIOLATION);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_province_code_must_be_two_capital_letters_and_unique() {
    let (_turn, pool) = setup().await;
    let insert = |code: &str| {
        format!(
            "INSERT INTO provinces (code, name, country_code, gst_rate_thousandths, effective_from, note) \
             VALUES ('{code}', 'Test Land {code}', 'CA', 5000, '2026-01-01', 'test')"
        )
    };
    assert_eq!(refused(&pool, &insert("sk")).await, CHECK_VIOLATION);
    assert_eq!(refused(&pool, &insert("SKK")).await, CHECK_VIOLATION);
    assert_eq!(refused(&pool, &insert("SK")).await, UNIQUE_VIOLATION);
}

// ===== a province is never deleted =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_province_cannot_be_deleted() {
    let (_turn, pool) = setup().await;
    assert_eq!(
        refused(&pool, "DELETE FROM provinces WHERE code = 'SK'").await,
        RESTRICT_VIOLATION
    );
    assert_eq!(
        refused(&pool, "DELETE FROM provinces").await,
        RESTRICT_VIOLATION
    );
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_provinces_table_cannot_be_emptied_with_truncate() {
    let (_turn, pool) = setup().await;
    assert_eq!(
        refused(&pool, "TRUNCATE provinces CASCADE").await,
        RESTRICT_VIOLATION
    );
}

// ===== changing a rate (what the admin does) =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn changing_a_rate_keeps_the_old_values_in_the_history() {
    let (_turn, pool) = setup().await;
    let mut tx = work(&pool).await;
    sqlx::query(
        "UPDATE provinces SET pst_rate_thousandths = 7000, effective_from = '2027-04-01', \
         note = 'test change' WHERE code = 'SK'",
    )
    .execute(&mut *tx)
    .await
    .unwrap();

    // The province now has the new values ...
    assert_eq!(rates(&mut tx, "SK").await, (Some(5000), Some(7000), None));
    // ... and the log holds the OLD ones, with their own date and note.
    let rows = count(
        &mut tx,
        "SELECT count(*) FROM province_rate_history WHERE province_code = 'SK'",
    )
    .await;
    assert_eq!(rows, 1);
    let (gst, pst, hst, from, note): (Option<i32>, Option<i32>, Option<i32>, String, String) =
        sqlx::query_as(
            "SELECT gst_rate_thousandths, pst_rate_thousandths, hst_rate_thousandths, \
             effective_from::text, note FROM province_rate_history WHERE province_code = 'SK'",
        )
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!((gst, pst, hst), (Some(5000), Some(6000), None));
    assert_eq!(from, "2025-04-01");
    assert_eq!(note, "CRA GST/HST rates by province, checked 2026-11-17");
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_province_can_switch_from_gst_and_pst_to_hst_in_one_update() {
    let (_turn, pool) = setup().await;
    let mut tx = work(&pool).await;
    sqlx::query(
        "UPDATE provinces SET gst_rate_thousandths = NULL, pst_rate_thousandths = NULL, \
         hst_rate_thousandths = 11000, effective_from = '2027-01-01', note = 'test switch' \
         WHERE code = 'SK'",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    assert_eq!(rates(&mut tx, "SK").await, (None, None, Some(11000)));
    let old_pst: Option<i32> = sqlx::query_scalar(
        "SELECT pst_rate_thousandths FROM province_rate_history WHERE province_code = 'SK'",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(old_pst, Some(6000));
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn two_changes_make_two_history_rows() {
    let (_turn, pool) = setup().await;
    let mut tx = work(&pool).await;
    for (pst, from) in [(6500, "2027-01-01"), (7000, "2028-01-01")] {
        sqlx::query(
            "UPDATE provinces SET pst_rate_thousandths = $1, effective_from = $2::date, \
             note = 'test' WHERE code = 'SK'",
        )
        .bind(pst)
        .bind(from)
        .execute(&mut *tx)
        .await
        .unwrap();
    }
    let old_psts: Vec<Option<i32>> = sqlx::query_scalar(
        "SELECT pst_rate_thousandths FROM province_rate_history \
         WHERE province_code = 'SK' ORDER BY effective_from",
    )
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    assert_eq!(old_psts, [Some(6000), Some(6500)]);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn fixing_a_name_or_changing_only_the_note_writes_no_history() {
    let (_turn, pool) = setup().await;
    let mut tx = work(&pool).await;
    sqlx::query(
        "UPDATE provinces SET name = 'Saskatchewan (test)', note = 'only a note' WHERE code = 'SK'",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    assert_eq!(
        count(&mut tx, "SELECT count(*) FROM province_rate_history").await,
        0
    );
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_refused_change_leaves_no_history_behind() {
    let (_turn, pool) = setup().await;
    let mut tx = work(&pool).await;
    let result = sqlx::query("UPDATE provinces SET hst_rate_thousandths = 13000 WHERE code = 'AB'")
        .execute(&mut *tx)
        .await;
    assert!(result.is_err());
    // The transaction is spoiled by the error; the log is checked from a fresh one.
    drop(tx);
    let mut fresh = work(&pool).await;
    assert_eq!(
        count(
            &mut fresh,
            "SELECT count(*) FROM province_rate_history WHERE province_code = 'AB'"
        )
        .await,
        0
    );
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn updated_at_moves_when_a_row_is_updated() {
    let (_turn, pool) = setup().await;
    let mut tx = work(&pool).await;
    let moved: bool = sqlx::query_scalar(
        "WITH changed AS (UPDATE provinces SET note = 'touched' WHERE code = 'SK' \
         RETURNING updated_at, created_at) SELECT updated_at > created_at FROM changed",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert!(moved);
}

// ===== the history is a log =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_history_cannot_be_edited_or_deleted() {
    let (_turn, pool) = setup().await;
    // The log is empty in a clean database, so each attempt first adds a row by hand
    // (inside its own transaction, which is rolled back), then tries to touch it.
    let insert = "INSERT INTO province_rate_history (province_code, gst_rate_thousandths, effective_from, note) \
                  VALUES ('SK', 5000, '2025-04-01', 'test')";
    for statement in [
        "UPDATE province_rate_history SET note = 'edited'",
        "DELETE FROM province_rate_history",
        "TRUNCATE province_rate_history",
    ] {
        let mut tx = work(&pool).await;
        sqlx::query(insert).execute(&mut *tx).await.unwrap();
        let error = sqlx::query(statement).execute(&mut *tx).await.unwrap_err();
        assert_eq!(sqlstate(error), RESTRICT_VIOLATION, "{statement}");
    }
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn history_can_only_point_at_a_real_province() {
    let (_turn, pool) = setup().await;
    let sql = "INSERT INTO province_rate_history (province_code, gst_rate_thousandths, effective_from, note) \
               VALUES ('ZZ', 5000, '2025-04-01', 'test')";
    assert_eq!(refused(&pool, sql).await, "23503"); // foreign key violation
}
