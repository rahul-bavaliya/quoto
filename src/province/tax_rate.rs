// src/province/tax_rate.rs: one tax rate that is always allowed

use crate::errors::{QuoteError, QuoteResult};

/// The most a rate can be: 100%, in thousandths of a percent.
const MAX_THOUSANDTHS: i32 = 100_000;

/// One tax rate, kept as whole **thousandths of a percent** (5% is `5000`, 9.975% is `9975`),
/// so no decimal numbers are ever stored or compared (like money in cents).
///
/// The field is private and `from_thousandths` is the only way in, so anyone holding a
/// `TaxRate` knows it is above 0% and at most 100%: nobody has to check it again.
/// ("No tax" is not a rate of 0%: it is the absence of a rate.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaxRate(i32);

impl TaxRate {
    pub fn from_thousandths(value: i32) -> QuoteResult<Self> {
        if (1..=MAX_THOUSANDTHS).contains(&value) {
            Ok(TaxRate(value))
        } else {
            Err(QuoteError::InvalidTaxRate { value })
        }
    }

    pub fn thousandths(&self) -> i32 {
        self.0
    }
}
