// src/province_country.rs: defines Country, Province, tax rate metadata, and ProvinceCountry
// in one place so `province` and `country.rs` aren't needed as separate files.

mod table;

pub use table::{
    ProvinceTaxRecord, change_rates, get_province_tax_rates, list_province_tax_rates,
    seed_provinces,
};

use crate::errors::AppError;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Country {
    Canada,
}

impl Country {
    pub fn code(self) -> &'static str {
        match self {
            Self::Canada => "CA",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Canada => "Canada",
        }
    }
}

impl fmt::Display for Country {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Province {
    Alberta,
    BritishColumbia,
    Manitoba,
    NewBrunswick,
    NewfoundlandAndLabrador,
    NorthwestTerritories,
    NovaScotia,
    Nunavut,
    Ontario,
    PrinceEdwardIsland,
    Quebec,
    Saskatchewan,
    Yukon,
}

impl Province {
    /// Parses a two-letter province/territory code (case-insensitive, trimmed).
    pub fn from_code(code: &str) -> Result<Self, AppError> {
        match code.trim().to_ascii_uppercase().as_str() {
            "AB" => Ok(Self::Alberta),
            "BC" => Ok(Self::BritishColumbia),
            "MB" => Ok(Self::Manitoba),
            "NB" => Ok(Self::NewBrunswick),
            "NL" => Ok(Self::NewfoundlandAndLabrador),
            "NT" => Ok(Self::NorthwestTerritories),
            "NS" => Ok(Self::NovaScotia),
            "NU" => Ok(Self::Nunavut),
            "ON" => Ok(Self::Ontario),
            "PE" => Ok(Self::PrinceEdwardIsland),
            "QC" => Ok(Self::Quebec),
            "SK" => Ok(Self::Saskatchewan),
            "YT" => Ok(Self::Yukon),
            _ => Err(AppError::InvalidProvince {
                code: code.to_string(),
            }),
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Alberta => "AB",
            Self::BritishColumbia => "BC",
            Self::Manitoba => "MB",
            Self::NewBrunswick => "NB",
            Self::NewfoundlandAndLabrador => "NL",
            Self::NorthwestTerritories => "NT",
            Self::NovaScotia => "NS",
            Self::Nunavut => "NU",
            Self::Ontario => "ON",
            Self::PrinceEdwardIsland => "PE",
            Self::Quebec => "QC",
            Self::Saskatchewan => "SK",
            Self::Yukon => "YT",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Alberta => "Alberta",
            Self::BritishColumbia => "British Columbia",
            Self::Manitoba => "Manitoba",
            Self::NewBrunswick => "New Brunswick",
            Self::NewfoundlandAndLabrador => "Newfoundland and Labrador",
            Self::NorthwestTerritories => "Northwest Territories",
            Self::NovaScotia => "Nova Scotia",
            Self::Nunavut => "Nunavut",
            Self::Ontario => "Ontario",
            Self::PrinceEdwardIsland => "Prince Edward Island",
            Self::Quebec => "Quebec",
            Self::Saskatchewan => "Saskatchewan",
            Self::Yukon => "Yukon",
        }
    }

    pub fn country(self) -> Country {
        Country::Canada
    }
}

impl FromStr for Province {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_code(s)
    }
}

impl fmt::Display for Province {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

/// Tax rates expressed in thousandths of a percent (e.g., `5000` = 5.000%, `9975` = 9.975%).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaxRateInfo {
    pub gst_milli_pct: Option<i32>,
    pub pst_milli_pct: Option<i32>,
    pub hst_milli_pct: Option<i32>,
}

impl TaxRateInfo {
    fn validate_rate(rate: i32) -> Result<i32, AppError> {
        if rate <= 0 || rate > 100_000 {
            return Err(AppError::InvalidTaxRate { value: rate });
        }
        Ok(rate)
    }

    /// Validates Quoto's tax rule:
    /// - GST + optional PST (HST must be `None`), OR
    /// - HST alone (GST and PST must be `None`).
    pub fn new(
        gst_milli_pct: Option<i32>,
        pst_milli_pct: Option<i32>,
        hst_milli_pct: Option<i32>,
    ) -> Result<Self, AppError> {
        match (gst_milli_pct, pst_milli_pct, hst_milli_pct) {
            (Some(gst), pst, None) => {
                let gst = Self::validate_rate(gst)?;
                let pst = pst.map(Self::validate_rate).transpose()?;
                Ok(Self {
                    gst_milli_pct: Some(gst),
                    pst_milli_pct: pst,
                    hst_milli_pct: None,
                })
            }
            (None, None, Some(hst)) => {
                let hst = Self::validate_rate(hst)?;
                Ok(Self {
                    gst_milli_pct: None,
                    pst_milli_pct: None,
                    hst_milli_pct: Some(hst),
                })
            }
            _ => Err(AppError::InvalidTaxSystem),
        }
    }

    pub fn total_milli_pct(&self) -> i32 {
        if let Some(hst) = self.hst_milli_pct {
            return hst;
        }
        self.gst_milli_pct.unwrap_or(0) + self.pst_milli_pct.unwrap_or(0)
    }
}

/// Holds a province, its country (derived from `Province`), and optional tax rate info.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProvinceCountry {
    province: Province,
    country: Country,
    tax_rates: Option<TaxRateInfo>,
}

impl ProvinceCountry {
    pub fn new(province: Province) -> Self {
        let country = province.country();
        Self {
            province,
            country,
            tax_rates: None,
        }
    }

    pub fn from_province_code(code: &str) -> Result<Self, AppError> {
        let province = Province::from_code(code)?;
        Ok(Self::new(province))
    }

    pub fn with_tax_rates(mut self, tax_rates: TaxRateInfo) -> Self {
        self.tax_rates = Some(tax_rates);
        self
    }

    pub fn province(&self) -> Province {
        self.province
    }

    pub fn province_code(&self) -> &'static str {
        self.province.code()
    }

    pub fn country(&self) -> Country {
        self.country
    }

    pub fn country_code(&self) -> &'static str {
        self.country.code()
    }

    pub fn tax_rates(&self) -> Option<TaxRateInfo> {
        self.tax_rates
    }
}

impl From<Province> for ProvinceCountry {
    fn from(province: Province) -> Self {
        Self::new(province)
    }
}

impl fmt::Display for ProvinceCountry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, {}", self.province_code(), self.country_code())
    }
}
