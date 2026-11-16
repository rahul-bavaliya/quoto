// src/quote/quote_number.rs

use crate::errors::QuoteError;
use std::fmt;

/// A quote number like "Q-2026-0001": the letter Q, the year, and a
/// counter that starts at 1 and restarts every year.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuoteNumber {
    year: u16,
    counter: u16,
}

impl QuoteNumber {
    /// A 4-digit year, and a counter from 1 to 9999.
    pub fn new(year: u16, counter: u16) -> Result<Self, QuoteError> {
        let year_ok = (1000..=9999).contains(&year);
        let counter_ok = (1..=9999).contains(&counter);
        if !year_ok || !counter_ok {
            return Err(QuoteError::InvalidQuoteNumber {
                input: format!("Q-{}-{:04}", year, counter),
            });
        }
        Ok(QuoteNumber { year, counter })
    }

    /// The first quote of a year.
    pub fn first_of_year(year: u16) -> Result<Self, QuoteError> {
        QuoteNumber::new(year, 1)
    }

    /// The next quote in the same year. After 9999 there is no next one:
    /// we refuse instead of wrapping around to a number already used.
    pub fn next(&self) -> Result<Self, QuoteError> {
        if self.counter == 9999 {
            return Err(QuoteError::QuoteNumbersExhausted { year: self.year });
        }
        QuoteNumber::new(self.year, self.counter + 1)
    }

    pub fn year(&self) -> u16 {
        self.year
    }

    pub fn counter(&self) -> u16 {
        self.counter
    }
}

/// e.g. "Q-2026-0001"
impl fmt::Display for QuoteNumber {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Q-{}-{:04}", self.year, self.counter)
    }
}
