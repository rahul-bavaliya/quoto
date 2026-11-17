// src/database.rs: the connection to PostgreSQL

use crate::errors::{QuoteError, QuoteResult};
use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
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
pub async fn connect(settings: &DatabaseSettings) -> QuoteResult<PgPool> {
    let options = PgConnectOptions::new()
        .host(&settings.host)
        .port(settings.port)
        .username(&settings.user)
        .password(settings.password.expose())
        .database(&settings.database)
        .ssl_mode(settings.ssl_mode);

    PgPoolOptions::new()
        .max_connections(MAX_CONNECTIONS)
        .acquire_timeout(WAIT_FOR_CONNECTION)
        .connect_with(options)
        .await
        .map_err(|cause| {
            // The cause helps whoever runs the server; the person sees only the
            // short message. Debug level, so it isn't printed by default.
            debug!(%cause, "database connection failed");
            QuoteError::DatabaseUnavailable
        })
}

/// Builds or updates the tables: runs every file in `migrations/` that hasn't run yet,
/// in order. The files are packed into the program when it is built, so a running
/// server can't be tricked into running a different file. Running it twice is safe
/// (a migration that already ran is skipped). It needs a database user that is allowed
/// to create tables; later the running program gets a user with fewer rights.
pub async fn migrate(pool: &PgPool) -> QuoteResult<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|cause| {
            debug!(%cause, "database migration failed");
            QuoteError::MigrationFailed
        })
}
