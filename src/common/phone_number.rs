// src/common/phone_number.rs

use crate::errors::AppError;
use std::fmt;

/// A North American phone number. Always stored as exactly 10 digits
/// ("3065550142") and shown as "+1-306-555-0142".
#[derive(Debug, PartialEq)]
pub struct PhoneNumber {
    digits: String,
}

impl PhoneNumber {
    /// Accepts messy input like "(306) 555 0142" or "+1 306 555 0142".
    pub fn parse(input: &str) -> Result<Self, AppError> {
        let mut digits: String = input.chars().filter(|c| c.is_ascii_digit()).collect();

        if digits.len() == 11 && digits.starts_with('1') {
            digits.remove(0);
        }

        let has_letters = input.chars().any(|c| c.is_alphabetic());
        if digits.len() != 10 || has_letters {
            return Err(AppError::InvalidPhone {
                input: input.to_string(),
            });
        }
        Ok(PhoneNumber { digits })
    }
}

impl fmt::Display for PhoneNumber {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let d = &self.digits;
        write!(f, "+1-{}-{}-{}", &d[..3], &d[3..6], &d[6..])
    }
}
