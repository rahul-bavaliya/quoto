// src/database.rs: the connection to PostgreSQL, and changing its tables (migrations)

use crate::errors::{QuoteError, QuoteResult};
use crate::{branch, customer, equipment, province, quote, repair_order};
use sea_orm::sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sea_orm::{DatabaseConnection, SqlxPostgresConnector};
use sea_orm_migration::MigratorTrait;
use std::time::Duration;
use tracing::debug;

mod migrations;
mod settings;

pub use settings::DatabaseSettings;

/// How many connections the pool keeps open at most, and how long we wait for one.
const MAX_CONNECTIONS: u32 = 5;
const WAIT_FOR_CONNECTION: Duration = Duration::from_secs(5);

/// Opens a pool of connections (a few phones kept on the desk, so every
/// request doesn't have to dial again). It connects right away, so a wrong
/// password or a stopped database is found now, not at the first quote.
///
/// The password is only opened here, with `expose()`. It is never logged.
pub async fn connect(settings: &DatabaseSettings) -> QuoteResult<DatabaseConnection> {
    let options = PgConnectOptions::new()
        .host(&settings.host)
        .port(settings.port)
        .username(&settings.user)
        .password(settings.password.expose())
        .database(&settings.database)
        .ssl_mode(settings.ssl_mode);

    let pool = PgPoolOptions::new()
        .max_connections(MAX_CONNECTIONS)
        .acquire_timeout(WAIT_FOR_CONNECTION)
        .connect_with(options)
        .await
        .map_err(|cause| {
            // The cause helps whoever runs the server; the person sees only the
            // short message. Debug level, so it isn't printed by default.
            debug!(%cause, "database connection failed");
            QuoteError::DatabaseUnavailable
        })?;
    Ok(SqlxPostgresConnector::from_sqlx_postgres_pool(pool))
}

/// Brings the database up to date: runs every migration it has not run yet, in order (see
/// `migrations.rs`). Running it again does nothing. It is the one way the tables change. It needs
/// a database user that is allowed to create and change tables.
pub async fn migrate(db: &DatabaseConnection) -> QuoteResult<()> {
    migrations::Migrator::up(db, None).await.map_err(|cause| {
        debug!(%cause, "database migration failed");
        QuoteError::DatabaseSetupFailed
    })
}

/// Makes the database match the Rust structs by creating a missing table or column. It never
/// drops anything. This is NOT how the server's database is built (that is `migrate`): it is
/// kept as the reference the tests compare the migrations against, so a struct and its table
/// can't drift apart unnoticed.
pub async fn sync(db: &DatabaseConnection) -> QuoteResult<()> {
    db.get_schema_builder()
        .register(province::ProvinceTable)
        .register(branch::BranchTable)
        .register(customer::CustomerTable)
        .register(equipment::EquipmentTable)
        .register(quote::QuoteNumberTable)
        .register(quote::QuoteTable)
        .register(quote::QuoteLineTable)
        .register(repair_order::RepairOrderNumberTable)
        .register(repair_order::RepairOrderTable)
        .sync(db)
        .await
        .map_err(|cause| {
            debug!(%cause, "database sync failed");
            QuoteError::DatabaseSetupFailed
        })
}
