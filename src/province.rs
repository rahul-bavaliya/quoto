// src/province.rs: a province, its country and its tax rates
//
// The pieces live in `src/province/`: `tax` (the tax sum and the rules for tax rates) and
// `table` (the `provinces` table in the database: build it, seed it, read and change the rates).

use crate::country::Country;

mod table;
mod tax;

pub(crate) use table::ProvinceTable;
pub use table::{all_rates, change_rates, rates_for, seed_provinces};
pub use tax::{TaxRate, TaxRates, calculate_tax};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Province {
    Saskatchewan,
    Alberta,
}

impl Province {
    pub fn label(&self) -> &str {
        match self {
            Province::Saskatchewan => "Saskatchewan",
            Province::Alberta => "Alberta",
        }
    }

    /// The two-letter code used on addresses, e.g. "SK".
    pub fn code(&self) -> &str {
        match self {
            Province::Saskatchewan => "SK",
            Province::Alberta => "AB",
        }
    }

    /// Every province belongs to one country.
    pub fn country(&self) -> Country {
        match self {
            Province::Saskatchewan | Province::Alberta => Country::Canada,
        }
    }

    pub fn gst_rate(&self) -> f64 {
        5.0
    }

    pub fn pst_rate(&self) -> Option<f64> {
        match self {
            Province::Saskatchewan => Some(6.0),
            Province::Alberta => None,
        }
    }
}
