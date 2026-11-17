// src/province/tax_rates.rs: the taxes a province charges

use super::tax_rate::TaxRate;
use crate::errors::{QuoteError, QuoteResult};

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
