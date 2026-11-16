// src/quote/quote_number.rs

use crate::errors::QuoteError;
use std::fmt;

/// A quote number like "Q-2026-0001": the letter Q, the year, and a
/// 4-digit counter that starts at 0001 and restarts every year.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuoteNumber {
    year: u16,
    counter: u16,
}

impl QuoteNumber {
    const FIRST_COUNTER: u16 = 1;
    const LAST_COUNTER: u16 = 9999;

    /// The one place the rules live: a 4-digit year, and a counter from 1 to 9999.
    pub fn new(year: u16, counter: u16) -> Result<Self, QuoteError> {
        let year_ok = (1000..=9999).contains(&year);
        let counter_ok = (Self::FIRST_COUNTER..=Self::LAST_COUNTER).contains(&counter);
        if !year_ok || !counter_ok {
            return Err(QuoteError::InvalidQuoteNumber {
                input: format!("Q-{}-{:04}", year, counter),
            });
        }
        Ok(QuoteNumber { year, counter })
    }

    /// The first quote of a year: counter 1.
    pub fn first_of_year(year: u16) -> Result<Self, QuoteError> {
        QuoteNumber::new(year, Self::FIRST_COUNTER)
    }

    /// The next quote in the same year. After 9999 there is no next one:
    /// we refuse instead of wrapping around to a number already used.
    pub fn next(&self) -> Result<Self, QuoteError> {
        if self.counter >= Self::LAST_COUNTER {
            return Err(QuoteError::QuoteNumbersExhausted { year: self.year });
        }
        QuoteNumber::new(self.year, self.counter + 1)
    }

    /// Accepts "Q-2026-0001", and also tidies up " q-2026-0001 ".
    /// Use this for numbers that arrive as text (a database, a web request).
    pub fn parse(input: &str) -> Result<Self, QuoteError> {
        // One place that builds the error, so every refusal looks the same
        let refused = || QuoteError::InvalidQuoteNumber {
            input: input.to_string(),
        };

        // 1. Clean: no spaces around it, and a capital Q
        let text = input.trim().to_uppercase();

        // 2. Cut at the dashes: we need exactly "Q", year, counter
        let mut pieces = text.split('-');
        let (Some("Q"), Some(year), Some(counter), None) =
            (pieces.next(), pieces.next(), pieces.next(), pieces.next())
        else {
            return Err(refused());
        };

        // 3. Check: exactly 4 digits each (this also rules out letters and signs)
        let is_four_digits = |s: &str| s.len() == 4 && s.chars().all(|c| c.is_ascii_digit());
        if !is_four_digits(year) || !is_four_digits(counter) {
            return Err(refused());
        }

        // 4. Turn the text into numbers, then let `new` apply the rules
        let year: u16 = year.parse().map_err(|_| refused())?;
        let counter: u16 = counter.parse().map_err(|_| refused())?;
        QuoteNumber::new(year, counter).map_err(|_| refused())
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

    // ---- new ----

    #[test]
    fn new_builds_a_valid_number() {
        let number = QuoteNumber::new(2026, 1).unwrap();
        assert_eq!(number.year(), 2026);
        assert_eq!(number.counter(), 1);
    }

    #[test]
    fn new_refuses_counter_zero() {
        assert!(QuoteNumber::new(2026, 0).is_err());
    }

    #[test]
    fn new_refuses_a_counter_over_9999() {
        assert!(QuoteNumber::new(2026, 10000).is_err());
    }

    #[test]
    fn new_accepts_the_highest_counter() {
        assert!(QuoteNumber::new(2026, 9999).is_ok());
    }

    #[test]
    fn new_refuses_a_year_that_is_not_four_digits() {
        assert!(QuoteNumber::new(26, 1).is_err());
        assert!(QuoteNumber::new(10000, 1).is_err());
    }

    // ---- first_of_year and next ----

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

    // ---- parse ----

    #[test]
    fn a_normal_quote_number_is_accepted() {
        let number = QuoteNumber::parse("Q-2026-0001").unwrap();
        assert_eq!(number.year(), 2026);
        assert_eq!(number.counter(), 1);
    }

    #[test]
    fn it_prints_the_same_text_it_was_given() {
        let number = QuoteNumber::parse("Q-2026-0042").unwrap();
        assert_eq!(number.to_string(), "Q-2026-0042");
    }

    #[test]
    fn spaces_and_a_small_q_are_cleaned_up() {
        let number = QuoteNumber::parse("  q-2026-0007 ").unwrap();
        assert_eq!(number.to_string(), "Q-2026-0007");
    }

    #[test]
    fn counter_zero_is_refused() {
        assert!(QuoteNumber::parse("Q-2026-0000").is_err());
    }

    #[test]
    fn a_missing_q_is_refused() {
        assert!(QuoteNumber::parse("2026-0001").is_err());
    }

    #[test]
    fn a_wrong_letter_is_refused() {
        assert!(QuoteNumber::parse("R-2026-0001").is_err());
    }

    #[test]
    fn a_short_year_is_refused() {
        assert!(QuoteNumber::parse("Q-26-0001").is_err());
    }

    #[test]
    fn a_short_counter_is_refused() {
        assert!(QuoteNumber::parse("Q-2026-1").is_err());
    }

    #[test]
    fn a_long_counter_is_refused() {
        assert!(QuoteNumber::parse("Q-2026-00001").is_err());
    }

    #[test]
    fn letters_in_the_digits_are_refused() {
        assert!(QuoteNumber::parse("Q-20X6-0001").is_err());
    }

    #[test]
    fn a_sign_in_the_digits_is_refused() {
        assert!(QuoteNumber::parse("Q-+026-0001").is_err());
    }

    #[test]
    fn an_extra_piece_is_refused() {
        assert!(QuoteNumber::parse("Q-2026-0001-9").is_err());
    }

    #[test]
    fn an_empty_string_is_refused() {
        assert!(QuoteNumber::parse("").is_err());
    }

    #[test]
    fn the_error_remembers_what_was_typed() {
        let result = QuoteNumber::parse("Q-2026-1");
        assert_eq!(
            result,
            Err(QuoteError::InvalidQuoteNumber {
                input: String::from("Q-2026-1")
            })
        );
    }
}
