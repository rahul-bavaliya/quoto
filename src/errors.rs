// src/errors.rs: every way Quoto can refuse something

use crate::money::format_money;
use thiserror::Error;
use tracing::{error, warn};

/// The result type used across Quoto: `QuoteResult<T>` means "a T, or a QuoteError".
pub type QuoteResult<T> = Result<T, QuoteError>;

/// Every way Quoto can refuse something. Each kind carries the details
/// a screen (or a person) needs to explain what went wrong.
///
/// The messages are written for the person using Quoto. `thiserror` writes
/// the `Display` (and `std::error::Error`) code from the `#[error(...)]` lines.
#[derive(Debug, PartialEq, Error)]
pub enum QuoteError {
    // ---- Quote lines ----
    #[error("{description} quantity must be greater than zero.")]
    BadQuantity { description: String },

    #[error("{description} unit price can't be negative.")]
    NegativePrice { description: String },

    #[error(
        "Credit of {} is larger than the subtotal of {}.",
        format_money(*.credit_cents),
        format_money(*.subtotal_cents)
    )]
    CreditTooLarge {
        credit_cents: i64,
        subtotal_cents: i64,
    },

    // ---- Customer ----
    #[error("{field} is required.")]
    MissingField { field: &'static str },

    #[error("\"{input}\" is not a valid 10-digit phone number.")]
    InvalidPhone { input: String },

    #[error("\"{input}\" is not a valid email address.")]
    InvalidEmail { input: String },

    #[error("\"{input}\" is not a valid postal code (like S4P 3Y2).")]
    InvalidPostalCode { input: String },

    // ---- Equipment ----
    #[error("A serial number or a plate is required to identify the unit.")]
    MissingIdentification,

    #[error("Meter reading {reading} must be a number that is zero or more.")]
    InvalidMeterReading { reading: f64 },
}

/// Who can fix the problem? Screens and APIs use this to pick the right reaction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ErrorKind {
    /// The input is wrong (a typo, a missing field). The person types it again.
    InvalidInput,
    /// The input is fine on its own, but a business rule says no.
    BusinessRule,
}

impl QuoteError {
    pub fn kind(&self) -> ErrorKind {
        match self {
            QuoteError::CreditTooLarge { .. } => ErrorKind::BusinessRule,
            QuoteError::BadQuantity { .. }
            | QuoteError::NegativePrice { .. }
            | QuoteError::MissingField { .. }
            | QuoteError::InvalidPhone { .. }
            | QuoteError::InvalidEmail { .. }
            | QuoteError::InvalidPostalCode { .. }
            | QuoteError::MissingIdentification
            | QuoteError::InvalidMeterReading { .. } => ErrorKind::InvalidInput,
        }
    }

    /// A short, stable code. Logs, the web API (Phase 5) and screens can
    /// rely on it even if the wording of the message changes.
    pub fn code(&self) -> &'static str {
        match self {
            QuoteError::BadQuantity { .. } => "BAD_QUANTITY",
            QuoteError::NegativePrice { .. } => "NEGATIVE_PRICE",
            QuoteError::CreditTooLarge { .. } => "CREDIT_TOO_LARGE",
            QuoteError::MissingField { .. } => "MISSING_FIELD",
            QuoteError::InvalidPhone { .. } => "INVALID_PHONE",
            QuoteError::InvalidEmail { .. } => "INVALID_EMAIL",
            QuoteError::InvalidPostalCode { .. } => "INVALID_POSTAL_CODE",
            QuoteError::MissingIdentification => "MISSING_IDENTIFICATION",
            QuoteError::InvalidMeterReading { .. } => "INVALID_METER_READING",
        }
    }

    /// The exit code the program ends with when this error stops it.
    pub fn exit_code(&self) -> u8 {
        match self.kind() {
            ErrorKind::InvalidInput => 2,
            ErrorKind::BusinessRule => 3,
        }
    }

    /// The one place that writes an error to the log.
    /// Wrong input is a warning (the person can fix it); a refused business
    /// rule is an error. Call this where the error finally stops (not at every step).
    ///
    /// Only the **code** is logged, never the message: messages can repeat what
    /// the customer typed (a phone number, an email), and logs must not hold that.
    pub fn log(&self) {
        match self.kind() {
            ErrorKind::InvalidInput => warn!(code = self.code(), "input refused"),
            ErrorKind::BusinessRule => error!(code = self.code(), "business rule refused"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credit_too_large_message_shows_dollars() {
        let error = QuoteError::CreditTooLarge {
            credit_cents: 30000,
            subtotal_cents: 19800,
        };
        assert_eq!(
            error.to_string(),
            "Credit of $300.00 is larger than the subtotal of $198.00."
        );
    }

    #[test]
    fn every_message_reads_like_a_sentence() {
        assert_eq!(
            QuoteError::BadQuantity {
                description: String::from("Oil Filter")
            }
            .to_string(),
            "Oil Filter quantity must be greater than zero."
        );
        assert_eq!(
            QuoteError::NegativePrice {
                description: String::from("Oil Filter")
            }
            .to_string(),
            "Oil Filter unit price can't be negative."
        );
        assert_eq!(
            QuoteError::MissingField { field: "city" }.to_string(),
            "city is required."
        );
        assert_eq!(
            QuoteError::InvalidPhone {
                input: String::from("555")
            }
            .to_string(),
            "\"555\" is not a valid 10-digit phone number."
        );
        assert_eq!(
            QuoteError::InvalidEmail {
                input: String::from("bob")
            }
            .to_string(),
            "\"bob\" is not a valid email address."
        );
        assert_eq!(
            QuoteError::InvalidPostalCode {
                input: String::from("12345")
            }
            .to_string(),
            "\"12345\" is not a valid postal code (like S4P 3Y2)."
        );
        assert_eq!(
            QuoteError::MissingIdentification.to_string(),
            "A serial number or a plate is required to identify the unit."
        );
        assert_eq!(
            QuoteError::InvalidMeterReading { reading: -5.0 }.to_string(),
            "Meter reading -5 must be a number that is zero or more."
        );
    }

    #[test]
    fn a_refused_credit_is_a_business_rule() {
        let error = QuoteError::CreditTooLarge {
            credit_cents: 100,
            subtotal_cents: 50,
        };
        assert_eq!(error.kind(), ErrorKind::BusinessRule);
        assert_eq!(error.exit_code(), 3);
    }

    #[test]
    fn a_typo_is_invalid_input() {
        let error = QuoteError::InvalidEmail {
            input: String::from("bob"),
        };
        assert_eq!(error.kind(), ErrorKind::InvalidInput);
        assert_eq!(error.exit_code(), 2);
    }

    #[test]
    fn every_error_has_its_own_code() {
        let codes = [
            QuoteError::BadQuantity {
                description: String::new(),
            }
            .code(),
            QuoteError::NegativePrice {
                description: String::new(),
            }
            .code(),
            QuoteError::CreditTooLarge {
                credit_cents: 0,
                subtotal_cents: 0,
            }
            .code(),
            QuoteError::MissingField { field: "x" }.code(),
            QuoteError::InvalidPhone {
                input: String::new(),
            }
            .code(),
            QuoteError::InvalidEmail {
                input: String::new(),
            }
            .code(),
            QuoteError::InvalidPostalCode {
                input: String::new(),
            }
            .code(),
            QuoteError::MissingIdentification.code(),
            QuoteError::InvalidMeterReading { reading: 0.0 }.code(),
        ];
        let mut unique = codes.to_vec();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), codes.len());
    }

    #[test]
    fn an_error_works_as_a_standard_error() {
        // thiserror also makes QuoteError a std::error::Error, so it can be
        // boxed, chained and used with `?` in code that doesn't know QuoteError.
        let error: Box<dyn std::error::Error> = Box::new(QuoteError::MissingIdentification);
        assert!(error.to_string().contains("serial number"));
    }
}
