// src/common/email.rs

use crate::errors::AppError;
use std::fmt;

/// A validated email address, stored normalized in lowercase.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Email {
    address: String,
}

impl Email {
    /// Parses and normalizes an email address (e.g., `"User@Example.COM"` -> `"user@example.com"`).
    pub fn parse(input: &str) -> Result<Self, AppError> {
        let trimmed = input.trim();

        if trimmed.is_empty() || trimmed.chars().any(|c| c.is_whitespace()) {
            return Err(AppError::InvalidEmail {
                input: input.to_string(),
            });
        }

        let mut parts = trimmed.split('@');
        let (Some(local), Some(domain), None) = (parts.next(), parts.next(), parts.next()) else {
            return Err(AppError::InvalidEmail {
                input: input.to_string(),
            });
        };

        if local.is_empty() || domain.is_empty() || !domain.contains('.') {
            return Err(AppError::InvalidEmail {
                input: input.to_string(),
            });
        }

        if domain.starts_with('.') || domain.ends_with('.') {
            return Err(AppError::InvalidEmail {
                input: input.to_string(),
            });
        }

        Ok(Email {
            address: trimmed.to_ascii_lowercase(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.address
    }
}

impl AsRef<str> for Email {
    fn as_ref(&self) -> &str {
        &self.address
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.address)
    }
}
