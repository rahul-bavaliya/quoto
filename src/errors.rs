// src/error.rs

use std::fmt; // ① at the very top (brings in Rust's formatting tools)

#[derive(Debug, PartialEq)] // ② the enum
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
    // ③ the Display badge, below the enum
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            QuoteError::BadQuantity { description } => {
                write!(f, "{} quantity must be greater than zero.", description)
            }
            // 👉 you write the NegativePrice arm here
            QuoteError::NegativePrice { description } => {
                write!(f, "{} price must be greater than zero.", description)
            }
            // 👉 you write the CreditTooLarge arm here
            QuoteError::CreditTooLarge {
                credit_cents,
                subtotal_cents,
            } => {
                write!(f, "Credit is over by {}", credit_cents + subtotal_cents)
            }
        }
    }
}
