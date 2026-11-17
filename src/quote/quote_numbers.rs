// src/quote/quote_numbers.rs: the "ticket machine" that hands out quote numbers

use super::QuoteNumber;
use crate::errors::QuoteResult;

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
    /// The year is handed in: the machine never looks at the clock.
    pub fn new(year: u16) -> Self {
        InMemoryQuoteNumbers { year, last: None }
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
