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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_builds_a_valid_number() {
        let number = QuoteNumber::new(2026, 1).unwrap();
        assert_eq!(number.year(), 2026);
        assert_eq!(number.counter(), 1);
    }

    #[test]
    fn it_prints_with_leading_zeros() {
        let number = QuoteNumber::new(2026, 42).unwrap();
        assert_eq!(number.to_string(), "Q-2026-0042");
    }

    #[test]
    fn counter_zero_is_refused() {
        assert!(QuoteNumber::new(2026, 0).is_err());
    }

    #[test]
    fn a_counter_over_9999_is_refused() {
        assert!(QuoteNumber::new(2026, 10000).is_err());
    }

    #[test]
    fn the_highest_counter_is_accepted() {
        assert!(QuoteNumber::new(2026, 9999).is_ok());
    }

    #[test]
    fn a_year_that_is_not_four_digits_is_refused() {
        assert!(QuoteNumber::new(26, 1).is_err());
        assert!(QuoteNumber::new(10000, 1).is_err());
    }

    #[test]
    fn the_error_remembers_the_number_it_refused() {
        assert_eq!(
            QuoteNumber::new(26, 1),
            Err(QuoteError::InvalidQuoteNumber {
                input: String::from("Q-26-0001")
            })
        );
    }

    #[test]
    fn the_first_quote_of_a_year_is_counter_one() {
        let number = QuoteNumber::first_of_year(2027).unwrap();
        assert_eq!(number.to_string(), "Q-2027-0001");
    }

    #[test]
    fn next_adds_one_in_the_same_year() {
        let number = QuoteNumber::new(2026, 41).unwrap();
        assert_eq!(number.next().unwrap().to_string(), "Q-2026-0042");
    }

    #[test]
    fn next_after_9999_is_refused_not_wrapped() {
        let last = QuoteNumber::new(2026, 9999).unwrap();
        assert_eq!(
            last.next(),
            Err(QuoteError::QuoteNumbersExhausted { year: 2026 })
        );
    }
}
