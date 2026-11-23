// src/common/postal_code.rs

use crate::errors::AppError;
use std::fmt;

/// A Canadian postal code, always stored as "A1A1A1" and shown as "A1A 1A1".
#[derive(Debug, PartialEq)]
pub struct PostalCode {
    value: String,
}

impl PostalCode {
    pub fn parse(input: &str) -> Result<Self, AppError> {
        let cleaned: String = input
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '-')
            .collect::<String>()
            .to_uppercase();

        let chars: Vec<char> = cleaned.chars().collect();
        let shape_ok = chars.len() == 6
            && chars.iter().enumerate().all(|(i, c)| {
                if i % 2 == 0 {
                    c.is_ascii_alphabetic()
                } else {
                    c.is_ascii_digit()
                }
            });

        if !shape_ok {
            return Err(AppError::InvalidPostalCode {
                input: input.to_string(),
            });
        }
        Ok(PostalCode { value: cleaned })
    }
}

impl fmt::Display for PostalCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", &self.value[..3], &self.value[3..])
    }
}
