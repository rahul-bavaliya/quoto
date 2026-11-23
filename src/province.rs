// src/province.rs: a province, its country and its tax rates
//
// The pieces live in `src/province/`: `tax` (the tax sum and the rules for tax rates) and
// `table` (the `provinces` table in the database: build it, seed it, read and change the rates).

use crate::country::Country;
use crate::errors::{AppError, AppResult};

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

    /// The other way round: finds a province by its two-letter code (as kept in the database).
    /// Only the provinces Quoto knows today are found; the enum grows to all 13 later.
    pub fn from_code(code: &str) -> AppResult<Province> {
        match code {
            "SK" => Ok(Province::Saskatchewan),
            "AB" => Ok(Province::Alberta),
            other => Err(AppError::UnknownProvince {
                code: other.to_string(),
            }),
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
