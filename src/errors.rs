// src/errors.rs: every way Quoto can refuse something

use crate::money::format_money;
use chrono::NaiveDate;
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
    #[error("\"{input}\" is not a valid quote number (like Q-2026-00000001).")]
    InvalidQuoteNumber { input: String },

    #[error("All the quote numbers for {year} are used up.")]
    QuoteNumbersExhausted { year: u16 },

    #[error("The expiry date {date} can't be before the quote date {quote_date}.")]
    ExpiryBeforeQuoteDate {
        date: NaiveDate,
        quote_date: NaiveDate,
    },

    #[error("The promise date {date} can't be before the quote date {quote_date}.")]
    PromiseBeforeQuoteDate {
        date: NaiveDate,
        quote_date: NaiveDate,
    },

    #[error("This quote is already {status}.")]
    AlreadyDecided { status: &'static str },

    #[error(
        "This quote was only valid until {valid_until}. Extend its expiry date before it can be accepted."
    )]
    QuoteExpired { valid_until: NaiveDate },

    #[error("Only a rejected quote can be reopened.")]
    NotRejected,

    #[error("The rejection reason can be at most {max} characters.")]
    RejectionReasonTooLong { max: usize },

    // ---- Equipment ----
    #[error("A serial number or a plate is required to identify the unit.")]
    MissingIdentification,

    #[error("Meter reading {reading} must be a number that is zero or more.")]
    InvalidMeterReading { reading: f64 },

    // ---- Outside services ----
    #[error("Could not look up the details with {provider}. Please enter them by hand.")]
    LookupFailed { provider: &'static str },

    #[error("Could not reach the database. Please try again later.")]
    DatabaseUnavailable,

    // ---- Settings (set by whoever runs the program, not typed by a customer) ----
    #[error("The setting {name} is missing. Copy .env.example to .env and fill it in.")]
    MissingSetting { name: &'static str },

    // The message never repeats the value: a setting may be (next to) a secret.
    #[error("The setting {name} is not valid.")]
    InvalidSetting { name: &'static str },
}

/// Who can fix the problem? Screens and APIs use this to pick the right reaction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ErrorKind {
    /// The input is wrong (a typo, a missing field). The person types it again.
    InvalidInput,
    /// The input is fine on its own, but a business rule says no.
    BusinessRule,
    /// An outside service didn't answer. Nobody typed anything wrong; try again later or enter by hand.
    ServiceUnavailable,
    /// The program is set up wrongly (a missing or invalid setting). Whoever runs it fixes it.
    Configuration,
}

impl QuoteError {
    pub fn kind(&self) -> ErrorKind {
        match self {
            QuoteError::CreditTooLarge { .. }
            | QuoteError::QuoteNumbersExhausted { .. }
            | QuoteError::AlreadyDecided { .. }
            | QuoteError::QuoteExpired { .. }
            | QuoteError::NotRejected => ErrorKind::BusinessRule,
            QuoteError::LookupFailed { .. } | QuoteError::DatabaseUnavailable => {
                ErrorKind::ServiceUnavailable
            }
            QuoteError::MissingSetting { .. } | QuoteError::InvalidSetting { .. } => {
                ErrorKind::Configuration
            }
            // Everything else is a wrong input.
            _ => ErrorKind::InvalidInput,
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
            QuoteError::ExpiryBeforeQuoteDate { .. } => "EXPIRY_BEFORE_QUOTE_DATE",
            QuoteError::PromiseBeforeQuoteDate { .. } => "PROMISE_BEFORE_QUOTE_DATE",
            QuoteError::AlreadyDecided { .. } => "ALREADY_DECIDED",
            QuoteError::QuoteExpired { .. } => "QUOTE_EXPIRED",
            QuoteError::NotRejected => "NOT_REJECTED",
            QuoteError::RejectionReasonTooLong { .. } => "REJECTION_REASON_TOO_LONG",
            QuoteError::MissingIdentification => "MISSING_IDENTIFICATION",
            QuoteError::InvalidMeterReading { .. } => "INVALID_METER_READING",
            QuoteError::LookupFailed { .. } => "LOOKUP_FAILED",
            QuoteError::DatabaseUnavailable => "DATABASE_UNAVAILABLE",
            QuoteError::MissingSetting { .. } => "MISSING_SETTING",
            QuoteError::InvalidSetting { .. } => "INVALID_SETTING",
        }
    }

    /// The exit code the program ends with when this error stops it.
    pub fn exit_code(&self) -> u8 {
        match self.kind() {
            ErrorKind::InvalidInput => 2,
            ErrorKind::BusinessRule => 3,
            ErrorKind::ServiceUnavailable => 4,
            ErrorKind::Configuration => 5,
        }
    }

    /// The one place that writes an error to the log.
    /// Wrong input and an unavailable outside service are warnings (the person
    /// can fix it or enter the details by hand); a refused business rule and a wrong
    /// set-up are errors. Call this where the error finally stops (not at every step).
    ///
    /// Only the **code** is logged, never the message: messages can repeat what
    /// the customer typed (a phone number, an email), and logs must not hold that.
    pub fn log(&self) {
        match self.kind() {
            ErrorKind::InvalidInput => warn!(code = self.code(), "input refused"),
            ErrorKind::BusinessRule => error!(code = self.code(), "business rule refused"),
            ErrorKind::Configuration => error!(code = self.code(), "program set up wrongly"),
            ErrorKind::ServiceUnavailable => {
                warn!(code = self.code(), "outside service unavailable")
            }
        }
    }
}
