// src/database/settings.rs: where the database is and how to log in

use crate::errors::{QuoteError, QuoteResult};
use crate::secret::Secret;
use sqlx::postgres::PgSslMode;

/// How to reach the database. Read from environment variables, never from code,
/// so no password is ever written in a file that goes to git.
///
/// The names are the same ones the Postgres Docker image uses, so
/// `docker-compose.yml` and Quoto read the **same** `.env` file (one source of truth).
#[derive(Debug)]
pub struct DatabaseSettings {
    pub(super) host: String,
    pub(super) port: u16,
    pub(super) user: String,
    pub(super) password: Secret,
    pub(super) database: String,
    pub(super) ssl_mode: PgSslMode,
}

impl DatabaseSettings {
    /// Reads the settings from the real environment variables.
    pub fn from_env() -> QuoteResult<Self> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// Reads the settings from any "name → value" source, so tests don't have
    /// to change the real environment.
    ///
    /// No default for the user, the password or the database name: a forgotten
    /// setting must stop the program, not fall back to a password anyone can guess.
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> QuoteResult<Self> {
        // A blank value counts as "not set".
        let value = |name: &str| {
            get(name)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let required = |name: &'static str| value(name).ok_or(QuoteError::MissingSetting { name });

        let port = match value("POSTGRES_PORT") {
            None => 5432,
            Some(text) => text.parse().map_err(|_| QuoteError::InvalidSetting {
                name: "POSTGRES_PORT",
            })?,
        };
        // "prefer" uses encryption when the server offers it. Production should
        // set POSTGRES_SSLMODE=require (or verify-full).
        let ssl_mode = match value("POSTGRES_SSLMODE") {
            None => PgSslMode::Prefer,
            Some(text) => text.parse().map_err(|_| QuoteError::InvalidSetting {
                name: "POSTGRES_SSLMODE",
            })?,
        };

        Ok(DatabaseSettings {
            host: value("POSTGRES_HOST").unwrap_or_else(|| String::from("localhost")),
            port,
            user: required("POSTGRES_USER")?,
            password: Secret::new(required("POSTGRES_PASSWORD")?),
            database: required("POSTGRES_DB")?,
            ssl_mode,
        })
    }

    /// Where we connect (safe to show: no user, no password).
    pub fn describe(&self) -> String {
        format!("{}:{}/{}", self.host, self.port, self.database)
    }
}
