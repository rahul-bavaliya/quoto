// src/quote/quote_number.rs

use crate::errors::QuoteError;
use std::fmt;

/// 8 digits: 99,999,999 quotes a year is more than enough.
const MAX_COUNTER: u32 = 99_999_999;

/// A quote number like "Q-2026-00000001": the letter Q, the year, and an
/// 8-digit counter that starts at 1 and restarts every year.
#[derive(Debug, Clone, PartialEq)]
pub struct QuoteNumber {
    year: u16,
    counter: u32,
}

impl QuoteNumber {
    /// A 4-digit year, and a counter from 1 to 99,999,999.
    pub fn new(year: u16, counter: u32) -> Result<Self, QuoteError> {
        let year_ok = (1000..=9999).contains(&year);
        let counter_ok = (1..=MAX_COUNTER).contains(&counter);
        if !year_ok || !counter_ok {
            return Err(QuoteError::InvalidQuoteNumber {
                input: format!("Q-{}-{:08}", year, counter),
            });
        }
        Ok(QuoteNumber { year, counter })
    }

    /// The first quote of a year.
    pub fn first_of_year(year: u16) -> Result<Self, QuoteError> {
        QuoteNumber::new(year, 1)
    }

    /// The next quote in the same year. After the last one there is no next one:
    /// we refuse instead of wrapping around to a number already used.
    pub fn next(&self) -> Result<Self, QuoteError> {
        if self.counter == MAX_COUNTER {
            return Err(QuoteError::QuoteNumbersExhausted { year: self.year });
        }
        QuoteNumber::new(self.year, self.counter + 1)
    }
}

/// e.g. "Q-2026-00000001"
impl fmt::Display for QuoteNumber {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Q-{}-{:08}", self.year, self.counter)
    }
}
