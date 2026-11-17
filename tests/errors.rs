// tests/errors.rs: tests for the errors feature (public API only)

use quoto::*;

// ===== errors =====

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
        QuoteError::InvalidQuoteNumber {
            input: String::from("Q-1")
        }
        .to_string(),
        "\"Q-1\" is not a valid quote number (like Q-2026-00000001)."
    );
    assert_eq!(
        QuoteError::QuoteNumbersExhausted { year: 2026 }.to_string(),
        "All the quote numbers for 2026 are used up."
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
fn running_out_of_quote_numbers_is_a_business_rule() {
    let error = QuoteError::QuoteNumbersExhausted { year: 2026 };
    assert_eq!(error.kind(), ErrorKind::BusinessRule);
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
        QuoteError::InvalidQuoteNumber {
            input: String::new(),
        }
        .code(),
        QuoteError::QuoteNumbersExhausted { year: 2026 }.code(),
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
