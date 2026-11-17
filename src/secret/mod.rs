// src/secret/mod.rs: a value that must never be shown (a password, an API key)

use std::fmt;

/// A piece of text that must never be printed or logged.
///
/// Think of a sealed envelope: you can carry it around, but printing it
/// (`{}` or `{:?}`) only shows "[hidden]". The only way to read the text is
/// the loud, easy-to-search call `expose()`, used at the one place that
/// really needs it (opening the database connection).
pub struct Secret(String);

impl Secret {
    pub fn new(text: String) -> Self {
        Secret(text)
    }

    /// The real text. Search for `expose(` to find every place that touches a secret.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Secret([hidden])")
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[hidden]")
    }
}
