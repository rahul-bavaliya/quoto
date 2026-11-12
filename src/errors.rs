// src/errors.rs

use crate::money::format_money;
use std::fmt;

/// Every way Quoto can refuse something. Each kind carries the details
/// a screen (or a person) needs to explain what went wrong.
#[derive(Debug, PartialEq)]
pub enum QuoteError {
    BadQuantity {
        description: String,
    },
    NegativePrice {
        description: String,
    },
    CreditTooLarge {
        credit_cents: i64,
        subtotal_cents: i64,
    },
}

impl fmt::Display for QuoteError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            QuoteError::BadQuantity { description } => {
                write!(f, "{} quantity must be greater than zero.", description)
            }
            QuoteError::NegativePrice { description } => {
                write!(f, "{} unit price can't be negative.", description)
            }
            QuoteError::CreditTooLarge {
                credit_cents,
                subtotal_cents,
            } => {
                write!(
                    f,
                    "Credit of {} is larger than the subtotal of {}.",
                    format_money(*credit_cents),
                    format_money(*subtotal_cents)
                )
            }
        }
    }
}

#[cfg(test)]
mod errors_tests {
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
}
