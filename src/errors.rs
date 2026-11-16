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

    // ---- Quote ----
    #[error("\"{input}\" is not a valid quote number (like Q-2026-0001).")]
    InvalidQuoteNumber { input: String },

    #[error("All the quote numbers for {year} are used up.")]
    QuoteNumbersExhausted { year: u16 },

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
            QuoteError::CreditTooLarge { .. } | QuoteError::QuoteNumbersExhausted { .. } => {
                ErrorKind::BusinessRule
            }
            QuoteError::BadQuantity { .. }
            | QuoteError::NegativePrice { .. }
            | QuoteError::MissingField { .. }
            | QuoteError::InvalidPhone { .. }
            | QuoteError::InvalidEmail { .. }
            | QuoteError::InvalidPostalCode { .. }
            | QuoteError::InvalidQuoteNumber { .. }
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
            QuoteError::InvalidQuoteNumber { .. } => "INVALID_QUOTE_NUMBER",
            QuoteError::QuoteNumbersExhausted { .. } => "QUOTE_NUMBERS_EXHAUSTED",
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
