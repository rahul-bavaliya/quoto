// src/errors.rs: every way Quoto can refuse something

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
    #[error("A line's quantity must be greater than zero.")]
    BadQuantity { description: String },

    #[error("A line's unit price can't be negative.")]
    NegativePrice { description: String },

    #[error("A line is too large.")]
    LineTooLarge { description: String },

    #[error("A line's description can be at most {max} characters.")]
    DescriptionTooLong { max: usize },

    #[error("A quote can have at most {max} lines.")]
    TooManyLines { max: usize },

    #[error("A line is a labour, part, fee or credit.")]
    InvalidLineKind,

    #[error(
        "The credit is larger than the subtotal."
    )]
    CreditTooLarge {
        credit_cents: i64,
        subtotal_cents: i64,
    },

    // ---- Customer ----
    #[error("{field} is required.")]
    MissingField { field: &'static str },

    #[error("That is not a valid 10-digit phone number.")]
    InvalidPhone { input: String },

    #[error("That is not a valid email address.")]
    InvalidEmail { input: String },

    #[error("That is not a valid postal code (like S4P 3Y2).")]
    InvalidPostalCode { input: String },

    // ---- Quote ----
    #[error("That is not a valid quote number (like Q-2026-00000001).")]
    InvalidQuoteNumber { input: String },

    #[error("All the quote numbers for {year} are used up.")]
    QuoteNumbersExhausted { year: u16 },

    #[error("The expiry date can't be before the quote date.")]
    ExpiryBeforeQuoteDate {
        date: NaiveDate,
        quote_date: NaiveDate,
    },

    #[error("The promise date can't be before the quote date.")]
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

    #[error("This quote already has a {what}.")]
    AlreadyAttached { what: &'static str },

    #[error("Add the customer before the equipment on a quote.")]
    EquipmentNeedsCustomer,

    #[error("This equipment belongs to a different customer.")]
    EquipmentNotOwnedByCustomer,

    #[error("A quote needs a {missing} before it can be accepted.")]
    QuoteIncomplete { missing: &'static str },

    #[error("A map location needs a latitude from -90 to 90 and a longitude from -180 to 180.")]
    InvalidLocation,

    #[error("This branch is inactive and cannot start new quotes.")]
    BranchInactive,

    #[error("The rejection reason can be at most {max} characters.")]
    RejectionReasonTooLong { max: usize },

    // ---- Provinces and taxes ----
    #[error(
        "A tax rate must be above 0% and at most 100% (1 to 100000 thousandths of a percent), not {value}."
    )]
    InvalidTaxRate { value: i32 },

    #[error("A province needs a GST (with an optional PST) or an HST: never both, never neither.")]
    InvalidTaxSystem,

    #[error("There is no province with that code.")]
    UnknownProvince { code: String },

    #[error("The new rates start on {date}, which is before the current rates ({current}).")]
    RatesDateBeforeCurrent { date: NaiveDate, current: NaiveDate },

    // ---- Web API ----
    #[error(
        "The web API could not start or keep running. Is another copy of Quoto already using its port? (RUST_LOG=debug shows the reason.)"
    )]
    ServerStartFailed,

    // ---- Customers ----
    #[error("There is no customer with that id.")]
    UnknownCustomer,

    #[error("This customer is already saved. To change it, use update.")]
    CustomerAlreadySaved,

    #[error("This customer is not saved yet, so it can't be updated. Create it first.")]
    CustomerNotSaved,

    #[error("This customer is inactive, so it can't be put on a new quote.")]
    CustomerInactive,

    // ---- Status and branches ----
    #[error("That is not a status. Use 'active' or 'inactive'.")]
    InvalidStatus { value: String },

    #[error("There is no branch with that id or number.")]
    UnknownBranch,

    #[error("A branch with that number already exists.")]
    DuplicateBranchNumber { number: String },

    #[error("This branch is already saved. To change it, use update.")]
    BranchAlreadySaved,

    #[error("This branch is not saved yet, so it can't be updated. Create it first.")]
    BranchNotSaved,

    // ---- Equipment ----
    #[error("A serial number or a plate is required to identify the unit.")]
    MissingIdentification,

    #[error("The meter reading must be a number that is zero or more.")]
    InvalidMeterReading { reading: f64 },

    #[error("That is not a meter type. Use 'km', 'mi' or 'hr'.")]
    InvalidMeterType { value: String },

    #[error("That is not an equipment type. Use car, truck, tractor, trailer or other.")]
    InvalidEquipmentType { value: String },

    #[error("The equipment's source is not one Quoto knows.")]
    InvalidEquipmentSource,

    #[error("There is no equipment with that id.")]
    UnknownEquipment,

    #[error("This equipment is already saved. To change it, use update.")]
    EquipmentAlreadySaved,

    #[error("This equipment is not saved yet, so it can't be updated. Create it first.")]
    EquipmentNotSaved,

    // ---- Saved quotes ----
    #[error("There is no quote with that id.")]
    UnknownQuote,

    #[error("This quote is already saved. To change it, use save_quote.")]
    QuoteAlreadySaved,

    #[error("This quote is not saved yet, so it can't be updated. Create it first.")]
    QuoteNotSaved,

    #[error(
        "This quote was changed by someone else since you opened it. Open it again and redo your change."
    )]
    QuoteChangedMeanwhile,

    #[error("A saved quote could not be read back. Ask whoever runs the server to check the logs.")]
    StoredQuoteDamaged,

    #[error("This equipment is archived, so it can't be put on a new quote. Restore it first.")]
    EquipmentInactive,

    // ---- Repair orders ----
    #[error("That is not a valid repair order number (like RO-2026-00000001).")]
    InvalidRepairOrderNumber { input: String },

    #[error("All the repair order numbers of {year} are used. Ask whoever runs the server.")]
    RepairOrderNumbersExhausted { year: u16 },

    #[error("There is no repair order with that id.")]
    UnknownRepairOrder,

    #[error("This repair order is not saved yet, so it can't be updated.")]
    RepairOrderNotSaved,

    #[error("Only an accepted quote can become a repair order.")]
    QuoteNotAccepted,

    #[error("This quote already has a repair order.")]
    RepairOrderExists,

    #[error(
        "A repair order needs the meter reading when the machine came in. Send it as meter_reading."
    )]
    MeterReadingRequired,

    #[error("Only an open repair order can be started.")]
    RepairOrderNotOpen,

    #[error("Only a repair order that is in progress can be completed.")]
    RepairOrderNotInProgress,

    #[error("This repair order is completed or cancelled, so it can't be changed.")]
    RepairOrderClosed,

    #[error("The due date can't be before the day the repair order was opened.")]
    DueDateBeforeOpened,

    #[error("The cancellation reason can be at most {max} characters.")]
    CancellationReasonTooLong { max: usize },

    /// `field` is one of Quoto's own words ("complaint", "cause", "correction"), never caller text.
    #[error("The {field} can be at most {max} characters.")]
    RepairOrderTextTooLong { field: &'static str, max: usize },

    #[error("The {field} can't hold control characters. New lines and tabs are fine.")]
    RepairOrderTextInvalid { field: &'static str },

    #[error(
        "This repair order was changed by someone else since you opened it. Open it again and redo your change."
    )]
    RepairOrderChangedMeanwhile,

    #[error(
        "A saved repair order could not be read back. Ask whoever runs the server to check the logs."
    )]
    StoredRepairOrderDamaged,

    #[error(
        "Equipment with this serial number already exists. Find it in the equipment list by serial number and change its owner instead of entering it again."
    )]
    DuplicateSerialNumber,

    // ---- Outside services ----
    #[error("Could not look up the details with {provider}. Please enter them by hand.")]
    LookupFailed { provider: &'static str },

    #[error("Could not reach the database. Please try again later.")]
    DatabaseUnavailable,

    #[error("The database could not be set up. Ask whoever runs the server to check the logs.")]
    DatabaseSetupFailed,

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
            | QuoteError::NotRejected
            | QuoteError::AlreadyAttached { .. }
            | QuoteError::EquipmentNeedsCustomer
            | QuoteError::EquipmentNotOwnedByCustomer
            | QuoteError::QuoteIncomplete { .. }
            | QuoteError::BranchInactive
            | QuoteError::DuplicateBranchNumber { .. }
            | QuoteError::BranchAlreadySaved
            | QuoteError::BranchNotSaved
            | QuoteError::CustomerAlreadySaved
            | QuoteError::CustomerNotSaved
            | QuoteError::CustomerInactive
            | QuoteError::EquipmentAlreadySaved
            | QuoteError::EquipmentNotSaved
            | QuoteError::EquipmentInactive
            | QuoteError::DuplicateSerialNumber
            | QuoteError::QuoteAlreadySaved
            | QuoteError::QuoteNotSaved
            | QuoteError::TooManyLines { .. }
            | QuoteError::QuoteChangedMeanwhile
            | QuoteError::RepairOrderNumbersExhausted { .. }
            | QuoteError::RepairOrderNotSaved
            | QuoteError::QuoteNotAccepted
            | QuoteError::RepairOrderExists
            | QuoteError::RepairOrderNotOpen
            | QuoteError::RepairOrderNotInProgress
            | QuoteError::RepairOrderClosed
            | QuoteError::RepairOrderChangedMeanwhile => ErrorKind::BusinessRule,
            QuoteError::LookupFailed { .. }
            | QuoteError::DatabaseUnavailable
            | QuoteError::DatabaseSetupFailed
            | QuoteError::StoredQuoteDamaged
            | QuoteError::StoredRepairOrderDamaged
            | QuoteError::ServerStartFailed => ErrorKind::ServiceUnavailable,
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
            QuoteError::LineTooLarge { .. } => "LINE_TOO_LARGE",
            QuoteError::DescriptionTooLong { .. } => "DESCRIPTION_TOO_LONG",
            QuoteError::TooManyLines { .. } => "TOO_MANY_LINES",
            QuoteError::InvalidLineKind => "INVALID_LINE_KIND",
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
            QuoteError::AlreadyAttached { .. } => "ALREADY_ATTACHED",
            QuoteError::EquipmentNeedsCustomer => "EQUIPMENT_NEEDS_CUSTOMER",
            QuoteError::EquipmentNotOwnedByCustomer => "EQUIPMENT_NOT_OWNED_BY_CUSTOMER",
            QuoteError::QuoteIncomplete { .. } => "QUOTE_INCOMPLETE",
            QuoteError::InvalidLocation => "INVALID_LOCATION",
            QuoteError::BranchInactive => "BRANCH_INACTIVE",
            QuoteError::RejectionReasonTooLong { .. } => "REJECTION_REASON_TOO_LONG",
            QuoteError::InvalidTaxRate { .. } => "INVALID_TAX_RATE",
            QuoteError::InvalidTaxSystem => "INVALID_TAX_SYSTEM",
            QuoteError::UnknownProvince { .. } => "UNKNOWN_PROVINCE",
            QuoteError::RatesDateBeforeCurrent { .. } => "RATES_DATE_BEFORE_CURRENT",
            QuoteError::ServerStartFailed => "SERVER_START_FAILED",
            QuoteError::UnknownCustomer => "UNKNOWN_CUSTOMER",
            QuoteError::CustomerAlreadySaved => "CUSTOMER_ALREADY_SAVED",
            QuoteError::CustomerNotSaved => "CUSTOMER_NOT_SAVED",
            QuoteError::CustomerInactive => "CUSTOMER_INACTIVE",
            QuoteError::InvalidStatus { .. } => "INVALID_STATUS",
            QuoteError::UnknownBranch => "UNKNOWN_BRANCH",
            QuoteError::DuplicateBranchNumber { .. } => "DUPLICATE_BRANCH_NUMBER",
            QuoteError::BranchAlreadySaved => "BRANCH_ALREADY_SAVED",
            QuoteError::BranchNotSaved => "BRANCH_NOT_SAVED",
            QuoteError::MissingIdentification => "MISSING_IDENTIFICATION",
            QuoteError::InvalidMeterReading { .. } => "INVALID_METER_READING",
            QuoteError::InvalidMeterType { .. } => "INVALID_METER_TYPE",
            QuoteError::InvalidEquipmentType { .. } => "INVALID_EQUIPMENT_TYPE",
            QuoteError::InvalidEquipmentSource => "INVALID_EQUIPMENT_SOURCE",
            QuoteError::UnknownEquipment => "UNKNOWN_EQUIPMENT",
            QuoteError::EquipmentAlreadySaved => "EQUIPMENT_ALREADY_SAVED",
            QuoteError::EquipmentNotSaved => "EQUIPMENT_NOT_SAVED",
            QuoteError::UnknownQuote => "UNKNOWN_QUOTE",
            QuoteError::QuoteAlreadySaved => "QUOTE_ALREADY_SAVED",
            QuoteError::QuoteNotSaved => "QUOTE_NOT_SAVED",
            QuoteError::QuoteChangedMeanwhile => "QUOTE_CHANGED_MEANWHILE",
            QuoteError::StoredQuoteDamaged => "STORED_QUOTE_DAMAGED",
            QuoteError::EquipmentInactive => "EQUIPMENT_INACTIVE",
            QuoteError::InvalidRepairOrderNumber { .. } => "INVALID_REPAIR_ORDER_NUMBER",
            QuoteError::RepairOrderNumbersExhausted { .. } => "REPAIR_ORDER_NUMBERS_EXHAUSTED",
            QuoteError::UnknownRepairOrder => "UNKNOWN_REPAIR_ORDER",
            QuoteError::RepairOrderNotSaved => "REPAIR_ORDER_NOT_SAVED",
            QuoteError::QuoteNotAccepted => "QUOTE_NOT_ACCEPTED",
            QuoteError::RepairOrderExists => "REPAIR_ORDER_EXISTS",
            QuoteError::MeterReadingRequired => "METER_READING_REQUIRED",
            QuoteError::RepairOrderNotOpen => "REPAIR_ORDER_NOT_OPEN",
            QuoteError::RepairOrderNotInProgress => "REPAIR_ORDER_NOT_IN_PROGRESS",
            QuoteError::RepairOrderClosed => "REPAIR_ORDER_CLOSED",
            QuoteError::DueDateBeforeOpened => "DUE_DATE_BEFORE_OPENED",
            QuoteError::CancellationReasonTooLong { .. } => "CANCELLATION_REASON_TOO_LONG",
            QuoteError::RepairOrderTextTooLong { .. } => "REPAIR_ORDER_TEXT_TOO_LONG",
            QuoteError::RepairOrderTextInvalid { .. } => "REPAIR_ORDER_TEXT_INVALID",
            QuoteError::RepairOrderChangedMeanwhile => "REPAIR_ORDER_CHANGED_MEANWHILE",
            QuoteError::StoredRepairOrderDamaged => "STORED_REPAIR_ORDER_DAMAGED",
            QuoteError::DuplicateSerialNumber => "DUPLICATE_SERIAL_NUMBER",
            QuoteError::LookupFailed { .. } => "LOOKUP_FAILED",
            QuoteError::DatabaseUnavailable => "DATABASE_UNAVAILABLE",
            QuoteError::DatabaseSetupFailed => "DATABASE_SETUP_FAILED",
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
