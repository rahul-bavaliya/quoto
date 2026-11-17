// src/province.rs: a province, its country and its tax rates
//
// The pieces live in `src/province/`: `tax` (the tax sum), `tax_rate` and `tax_rates`
// (the rules for the rates), `entity` (the `provinces` table as a struct; `database::sync`
// builds the table from it), `seed` (the 13 starting rows) and `store` (read and change rates).

use crate::country::Country;

pub(crate) mod entity;
mod seed;
mod store;
mod tax;
mod tax_rate;
mod tax_rates;

pub use seed::seed_provinces;
pub use store::{all_rates, change_rates, rates_for};
pub use tax::calculate_tax;
pub use tax_rate::TaxRate;
pub use tax_rates::TaxRates;

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
