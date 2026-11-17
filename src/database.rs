// src/database.rs: the connection to PostgreSQL, and building its tables

use crate::errors::{QuoteError, QuoteResult};
use crate::province;
use sea_orm::sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sea_orm::{DatabaseConnection, SqlxPostgresConnector};
use std::time::Duration;
use tracing::debug;

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

/// Makes the database match the Rust structs: creates a missing table, adds a missing column.
/// It never drops anything (a field removed from a struct leaves its column behind), and
/// running it twice is safe. For learning and development: it changes the database when the
/// program starts and writes no migration file, so before real customer data it is replaced by
/// reviewable migrations. It needs a database user that is allowed to create tables.
pub async fn sync(db: &DatabaseConnection) -> QuoteResult<()> {
    db.get_schema_builder()
        .register(province::ProvinceTable)
        .sync(db)
        .await
        .map_err(|cause| {
            debug!(%cause, "database sync failed");
            QuoteError::DatabaseSetupFailed
        })
}
