// src/province.rs: a province, its country and its tax rates
//
// The pieces live in `src/province/`: `tax` (the tax sum). The tax RATES per province are
// kept in the database table `provinces` (see `migrations/`).

use crate::country::Country;

mod tax;

pub use tax::calculate_tax;

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
