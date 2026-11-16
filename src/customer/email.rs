// src/customer/email.rs

use crate::errors::QuoteError;
use std::fmt;

/// An email address with a sensible shape, stored in lowercase.
/// (A perfect email check is impossible; real systems also send a
/// confirmation email.)
#[derive(Debug, PartialEq)]
pub struct Email {
    value: String,
}

impl Email {
    pub fn parse(input: &str) -> Result<Self, QuoteError> {
        let value = input.trim().to_lowercase();
        let invalid = || QuoteError::InvalidEmail {
            input: input.to_string(),
        };

        if value.contains(char::is_whitespace) {
            return Err(invalid());
        }
        // exactly one '@', with something on both sides
        let (local, domain) = value.split_once('@').ok_or_else(invalid)?;
        if local.is_empty() || domain.contains('@') {
            return Err(invalid());
        }
        // the domain needs a '.' that isn't the first or last character
        let dot_in_middle = domain
            .find('.')
            .is_some_and(|i| i > 0 && !domain.ends_with('.'));
        if !dot_in_middle {
            return Err(invalid());
        }
        Ok(Email { value })
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}
