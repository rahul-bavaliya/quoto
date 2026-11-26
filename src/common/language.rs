// src/common/language.rs (or src/customer/language.rs)

use crate::errors::{AppError, AppResult};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    English,
    French,
}

impl Language {
    /// Standard ISO-style language code used across the domain.
    pub fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::French => "fr",
        }
    }

    /// Human-readable label for display/UI.
    pub fn label(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::French => "French",
        }
    }

    /// Parses common English/French codes or names (case-insensitive, trimmed).
    pub fn parse(input: &str) -> AppResult<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "en" | "eng" | "english" => Ok(Self::English),
            "fr" | "fra" | "fre" | "french" | "francais" | "français" => Ok(Self::French),
            _ => Err(AppError::InvalidField { field: "language" }),
        }
    }
}

impl Default for Language {
    fn default() -> Self {
        Self::English
    }
}

impl FromStr for Language {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
