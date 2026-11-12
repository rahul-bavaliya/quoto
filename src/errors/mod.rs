// src/errors/mod.rs

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
mod tests;
