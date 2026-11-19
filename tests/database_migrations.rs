// tests/database_migrations.rs: the migrations build the tables the Rust structs describe
//
// These need a running database whose user may create databases (the compose one does), so they
// are skipped normally (and in CI). Run them by hand:
//
//     docker compose up -d
//     cargo test --test database_migrations -- --ignored
//
// Each test makes its own empty throwaway database (named `quoto_mig_...`), uses it, and drops
// it again. It never touches the database you work in.

use quoto::*;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};

/// The tables made before the quote tables, and all of them.
const OLD_TABLES: [&str; 4] = ["provinces", "branches", "customers", "equipment"];
const ALL_TABLES: [&str; 7] = [
    "provinces",
    "branches",
    "customers",
    "equipment",
    "quote_numbers",
    "quotes",
    "quote_lines",
];

fn settings_for(database: &str) -> DatabaseSettings {
    dotenvy::dotenv().ok();
    let database = database.to_string();
    DatabaseSettings::from_lookup(move |name| {
        if name == "POSTGRES_DB" {
            Some(database.clone())
        } else {
            std::env::var(name).ok()
        }
    })
    .expect("database settings (see .env.example)")
}

/// An empty database made for one test, and dropped by `finish`.
struct Throwaway {
    admin: DatabaseConnection,
    name: String,
    db: DatabaseConnection,
}

impl Throwaway {
    async fn new(tag: &str) -> Throwaway {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let name = format!("quoto_mig_{tag}_{}_{nanos}", std::process::id());
        // The "postgres" database always exists: connect there to make and drop the others.
        let admin = database::connect(&settings_for("postgres"))
            .await
            .expect("database");
        admin
            .execute_unprepared(&format!("CREATE DATABASE {name}"))
            .await
            .expect("create a throwaway database (the user needs CREATEDB)");
        let db = database::connect(&settings_for(&name))
            .await
            .expect("connect");
        Throwaway { admin, name, db }
    }

    async fn finish(self) {
        let _ = self.db.close().await;
        self.admin
            .execute_unprepared(&format!(
                "DROP DATABASE IF EXISTS {} WITH (FORCE)",
                self.name
            ))
            .await
            .expect("drop the throwaway database");
    }
}

/// Everything about the tables that a mistake could change: columns (type, nullable, default),
/// indexes and constraints. Names of indexes are left out, so only what they do is compared.
/// Foreign keys are left out: the structs don't declare them (the quote tests check them).
async fn shape(db: &DatabaseConnection, tables: &[&str]) -> Vec<String> {
    let list = tables
        .iter()
        .map(|table| format!("'{table}'"))
        .collect::<Vec<_>>()
        .join(",");
    let queries = [
        "SELECT table_name || '.' || column_name || ' ' || data_type || ' null=' || is_nullable \
         || ' default=' || coalesce(column_default, '-') \
         FROM information_schema.columns \
         WHERE table_schema = 'public' AND table_name IN ({list}) \
         ORDER BY 1",
        "SELECT tablename || ' ' || regexp_replace(indexdef, 'INDEX \\S+ ON', 'INDEX ON') \
         FROM pg_indexes WHERE schemaname = 'public' AND tablename IN ({list}) \
         ORDER BY 1",
        "SELECT conrelid::regclass::text || ' ' || contype::text || ' ' || pg_get_constraintdef(oid) \
         FROM pg_constraint WHERE connamespace = 'public'::regnamespace \
         AND contype <> 'f' AND conrelid::regclass::text IN ({list}) ORDER BY 1",
    ];
    let mut lines = Vec::new();
    for sql in queries {
        let rows = db
            .query_all_raw(Statement::from_string(
                db.get_database_backend(),
                sql.replace("{list}", &list),
            ))
            .await
            .expect("look at the tables");
        for row in rows {
            lines.push(row.try_get_by_index::<String>(0).expect("text"));
        }
    }
    lines
}

async fn migrations_run(db: &DatabaseConnection) -> i64 {
    let row = db
        .query_one_raw(Statement::from_string(
            db.get_database_backend(),
            "SELECT count(*) FROM seaql_migrations",
        ))
        .await
        .expect("look at the migration list")
        .expect("a row");
    row.try_get_by_index::<i64>(0).expect("a number")
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_migrations_build_exactly_the_tables_the_structs_describe() {
    let by_migrations = Throwaway::new("a").await;
    let by_structs = Throwaway::new("b").await;
    database::migrate(&by_migrations.db).await.expect("migrate");
    database::sync(&by_structs.db).await.expect("sync");

    let from_migrations = shape(&by_migrations.db, &ALL_TABLES).await;
    let from_structs = shape(&by_structs.db, &ALL_TABLES).await;
    by_migrations.finish().await;
    by_structs.finish().await;

    assert!(
        from_migrations.len() > 40,
        "the comparison saw too little: {from_migrations:?}"
    );
    assert_eq!(from_migrations, from_structs);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn running_the_migrations_again_changes_nothing() {
    let place = Throwaway::new("again").await;
    database::migrate(&place.db).await.expect("first run");
    let first = shape(&place.db, &ALL_TABLES).await;
    database::migrate(&place.db).await.expect("second run");
    let second = shape(&place.db, &ALL_TABLES).await;
    let remembered = migrations_run(&place.db).await;
    place.finish().await;

    assert_eq!(first, second);
    assert_eq!(remembered, 3); // the three steps so far, each remembered once
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_database_built_before_migrations_existed_adopts_them_and_keeps_its_rows() {
    let place = Throwaway::new("adopt").await;
    // The old way: tables built from the structs, with a row in one of them.
    database::sync(&place.db).await.expect("sync");
    // (sync also builds the quote tables now; an old database doesn't have them yet.)
    place
        .db
        .execute_unprepared("DROP TABLE quote_lines, quotes, quote_numbers")
        .await
        .expect("drop the quote tables");
    place
        .db
        .execute_unprepared(
            "INSERT INTO equipment (make, model, meter_type, equipment_type, source, status, serial_number) \
             VALUES ('Mig-Test', 'Old Row', 'hr', 'Tractor', 'manual', 'active', 'MIG-OLD-1')",
        )
        .await
        .expect("insert");
    let before = shape(&place.db, &OLD_TABLES).await;

    database::migrate(&place.db).await.expect("migrate");
    let after = shape(&place.db, &OLD_TABLES).await;
    let kept = place
        .db
        .query_one_raw(Statement::from_string(
            place.db.get_database_backend(),
            "SELECT count(*) FROM equipment WHERE serial_number = 'MIG-OLD-1'",
        ))
        .await
        .expect("count")
        .expect("a row")
        .try_get_by_index::<i64>(0)
        .expect("a number");
    place.finish().await;

    assert_eq!(before, after); // nothing about the tables changed
    assert_eq!(kept, 1); // and the row is still there
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_tables_made_by_migrations_work_with_the_code() {
    let place = Throwaway::new("use").await;
    database::migrate(&place.db).await.expect("migrate");
    seed_provinces(&place.db).await.expect("seed");
    let customer = create_customer(
        &place.db,
        &samples::customer_in(Province::Saskatchewan).unwrap(),
    )
    .await
    .expect("create a customer");
    let machine = samples::corolla()
        .unwrap()
        .with_customer_id(customer.id().unwrap());
    let saved = create_equipment(&place.db, &machine).await;
    let listed = list_equipment(&place.db, &EquipmentFilter::default(), 10, 0).await;
    place.finish().await;

    assert!(saved.is_ok(), "{saved:?}");
    assert_eq!(listed.expect("list").len(), 1);
}
