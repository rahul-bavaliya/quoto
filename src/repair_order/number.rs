// src/repair_order/number.rs: "RO-2026-00000001"

use crate::errors::QuoteError;
use std::fmt;

/// 8 digits: 99,999,999 repair orders a year is more than enough.
const MAX_COUNTER: u32 = 99_999_999;

/// A repair order number like "RO-2026-00000001": the letters RO, the year, and an 8-digit counter
/// that starts at 1 and restarts every year. The same idea as a quote number, with its own counter.
#[derive(Debug, Clone, PartialEq)]
pub struct RepairOrderNumber {
    year: u16,
    counter: u32,
}

impl RepairOrderNumber {
    /// A 4-digit year, and a counter from 1 to 99,999,999.
    pub fn new(year: u16, counter: u32) -> Result<Self, QuoteError> {
        let year_ok = (1000..=9999).contains(&year);
        let counter_ok = (1..=MAX_COUNTER).contains(&counter);
        if !year_ok || !counter_ok {
            return Err(QuoteError::InvalidRepairOrderNumber {
                input: format!("RO-{year}-{counter:08}"),
            });
        }
        Ok(RepairOrderNumber { year, counter })
    }

    /// Reads a number written as "RO-2026-00000001" (what `Display` prints). Anything else is refused.
    pub fn parse(input: &str) -> Result<Self, QuoteError> {
        let refuse = || QuoteError::InvalidRepairOrderNumber {
            input: input.to_string(),
        };
        let mut parts = input.split('-');
        let (Some("RO"), Some(year), Some(counter), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(refuse());
        };
        // Exactly 4 and 8 digits, so "RO-2026-1" and "RO-+2026-..." are not numbers.
        let digits = |text: &str, length: usize| {
            text.len() == length && text.bytes().all(|byte| byte.is_ascii_digit())
        };
        if !digits(year, 4) || !digits(counter, 8) {
            return Err(refuse());
        }
        let year = year.parse().map_err(|_| refuse())?;
        let counter = counter.parse().map_err(|_| refuse())?;
        RepairOrderNumber::new(year, counter).map_err(|_| refuse())
    }

    /// The year in the number.
    pub fn year(&self) -> u16 {
        self.year
    }

    /// The counter in the number (1 for the first repair order of the year).
    pub fn counter(&self) -> u32 {
        self.counter
    }
}

/// e.g. "RO-2026-00000001"
impl fmt::Display for RepairOrderNumber {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RO-{}-{:08}", self.year, self.counter)
    }
}
