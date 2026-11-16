// src/customer/postal_code.rs

use crate::errors::QuoteError;
use std::fmt;

/// A Canadian postal code, always stored as "A1A1A1" and shown as "A1A 1A1".
#[derive(Debug, PartialEq)]
pub struct PostalCode {
    value: String,
}

impl PostalCode {
    pub fn parse(input: &str) -> Result<Self, QuoteError> {
        // 1. Clean: drop spaces and dashes, make letters UPPERCASE
        let cleaned: String = input
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '-')
            .collect::<String>()
            .to_uppercase();

        // 2. Check the shape: letter, digit, letter, digit, letter, digit
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
            return Err(QuoteError::InvalidPostalCode {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postal_code_is_cleaned_up() {
        let code = PostalCode::parse(" s4p-3y2 ").unwrap();
        assert_eq!(code.to_string(), "S4P 3Y2");
    }

    #[test]
    fn postal_code_with_wrong_shape_is_refused() {
        let result = PostalCode::parse("12345");
        assert!(matches!(result, Err(QuoteError::InvalidPostalCode { .. })));
    }

    #[test]
    fn postal_code_too_short_is_refused() {
        assert!(PostalCode::parse("S4P 3Y").is_err());
    }
}
