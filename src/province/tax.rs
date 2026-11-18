// src/province/tax.rs: the tax sum, and the rules for tax rates

use crate::errors::{QuoteError, QuoteResult};

/// Tax on an amount (in cents) at a percentage rate, rounded to the nearest cent.
pub fn calculate_tax(amount: i64, rate: f64) -> i64 {
    (amount as f64 * rate / 100.0).round() as i64
}

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

/// The tax system of one province. It is an `enum` so a wrong combination can't even be written:
/// a province has either a GST (with a PST or without one) **or** an HST on its own.
/// (Quebec's QST and Manitoba's RST are kept as the `pst`.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaxRates {
    GstPst { gst: TaxRate, pst: Option<TaxRate> },
    Hst(TaxRate),
}

impl TaxRates {
    /// Builds the taxes from the three database columns, and **refuses a bad row**
    /// (GST with HST, an HST with a PST, or no tax at all), so a damaged row is caught
    /// when it is loaded and never reaches a quote.
    pub fn from_columns(gst: Option<i32>, pst: Option<i32>, hst: Option<i32>) -> QuoteResult<Self> {
        let rate = |value: Option<i32>| value.map(TaxRate::from_thousandths).transpose();
        match (rate(gst)?, rate(pst)?, rate(hst)?) {
            (Some(gst), pst, None) => Ok(TaxRates::GstPst { gst, pst }),
            (None, None, Some(hst)) => Ok(TaxRates::Hst(hst)),
            _ => Err(QuoteError::InvalidTaxSystem),
        }
    }

    /// The three database columns again: `(gst, pst, hst)`.
    pub fn to_columns(&self) -> (Option<i32>, Option<i32>, Option<i32>) {
        match self {
            TaxRates::GstPst { gst, pst } => (
                Some(gst.thousandths()),
                pst.map(|rate| rate.thousandths()),
                None,
            ),
            TaxRates::Hst(hst) => (None, None, Some(hst.thousandths())),
        }
    }
}
