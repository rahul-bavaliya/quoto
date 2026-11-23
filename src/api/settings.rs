// src/api/settings.rs: where the web API listens and the key that opens it

use crate::errors::{AppError, AppResult};
use crate::secret::Secret;
use std::net::{IpAddr, SocketAddr};

/// A key shorter than this is too easy to guess. `openssl rand -base64 32` makes 44 characters.
const MIN_KEY_LENGTH: usize = 32;

/// How the web API starts. Read from environment variables, like the database settings.
///
/// There is **no default key**: a forgotten `API_KEY` stops the program. The API only listens on
/// this computer (`127.0.0.1`) for now: the key travels as plain text, so it must not cross a
/// network until there is HTTPS (Phase 10).
#[derive(Debug)]
pub struct ApiSettings {
    pub(super) address: SocketAddr,
    pub(super) key: Secret,
}

impl ApiSettings {
    /// Reads the settings from the real environment variables.
    pub fn from_env() -> AppResult<Self> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// Reads the settings from any "name → value" source (tests don't touch the real environment).
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> AppResult<Self> {
        let value = |name: &str| {
            get(name)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };

        let key = value("API_KEY").ok_or(AppError::MissingSetting { name: "API_KEY" })?;
        if key.chars().count() < MIN_KEY_LENGTH {
            return Err(AppError::InvalidSetting { name: "API_KEY" });
        }

        let host: IpAddr = match value("API_HOST") {
            Some(text) => text
                .parse()
                .map_err(|_| AppError::InvalidSetting { name: "API_HOST" })?,
            None => IpAddr::from([127, 0, 0, 1]),
        };
        if !host.is_loopback() {
            return Err(AppError::InvalidSetting { name: "API_HOST" });
        }
        let port: u16 = match value("API_PORT") {
            Some(text) => text
                .parse()
                .map_err(|_| AppError::InvalidSetting { name: "API_PORT" })?,
            None => 3000,
        };

        Ok(ApiSettings {
            address: SocketAddr::new(host, port),
            key: Secret::new(key),
        })
    }

    /// Where the API will listen (shown at start-up; the key is never shown).
    pub fn address(&self) -> SocketAddr {
        self.address
    }
}
