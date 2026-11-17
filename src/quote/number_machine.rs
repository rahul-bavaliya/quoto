// src/quote/number_machine.rs: the "ticket machine" that hands out quote numbers

use super::QuoteNumber;
use crate::errors::QuoteResult;
use chrono::{Datelike, Local};

/// Anything that can hand out the next quote number.
/// Today a machine in memory does the job; in Phase 4 a database will,
/// and nothing else has to change.
pub trait QuoteNumbers {
    /// Pressing the button changes the machine (it remembers the number it
    /// gave out), so this needs `&mut self`.
    fn next_number(&mut self) -> QuoteResult<QuoteNumber>;
}

/// A ticket machine that keeps its count in memory. It starts again from
/// number 1 every time the program starts.
pub struct InMemoryQuoteNumbers {
    year: u16,
    /// `None` until the first press: nothing has been given out yet.
    last: Option<QuoteNumber>,
}

impl InMemoryQuoteNumbers {
    /// A new machine for the current year, read from the clock.
    pub fn new() -> Self {
        // A year that doesn't fit in a `u16` becomes 0, which `QuoteNumber`
        // refuses with a clear error on the first press.
        let year = u16::try_from(Local::now().year()).unwrap_or(0);
        InMemoryQuoteNumbers::for_year(year)
    }

    /// A new machine for a chosen year. Tests use this so they never
    /// depend on today's date.
    pub fn for_year(year: u16) -> Self {
        InMemoryQuoteNumbers { year, last: None }
    }
}

impl Default for InMemoryQuoteNumbers {
    fn default() -> Self {
        InMemoryQuoteNumbers::new()
    }
}

impl QuoteNumbers for InMemoryQuoteNumbers {
    fn next_number(&mut self) -> QuoteResult<QuoteNumber> {
        let number = match &self.last {
            None => QuoteNumber::first_of_year(self.year)?,
            Some(last) => last.next()?,
        };
        // The machine keeps a copy; the caller gets the ticket.
        self.last = Some(number.clone());
        Ok(number)
    }
}
