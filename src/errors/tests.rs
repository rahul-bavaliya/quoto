// src/errors/tests.rs: unit tests for errors (they can see private fields)

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
