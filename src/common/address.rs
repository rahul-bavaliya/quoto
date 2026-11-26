// src/common/address.rs: where a party is (and which province's taxes apply)

use crate::common::PostalCode;
use crate::errors::AppError;
use crate::province_country::ProvinceCountry;

/// A mailing address. The province decides which taxes a quote charges.
#[derive(Debug, PartialEq)]
pub struct Address {
    line1: String,
    line2: Option<String>,
    city: String,
    postal_code: PostalCode,
    province_country: ProvinceCountry,
}

impl Address {
    pub fn new(
        line1: String,
        line2: Option<String>,
        city: String,
        postal_code: PostalCode,
        province_country: ProvinceCountry,
    ) -> Result<Self, AppError> {
        if line1.trim().is_empty() {
            return Err(AppError::MissingField {
                field: "address line 1",
            });
        }
        if city.trim().is_empty() {
            return Err(AppError::MissingField { field: "city" });
        }
        Ok(Address {
            line1,
            line2,
            city,
            postal_code,
            province_country,
        })
    }

    pub fn line1(&self) -> &str {
        &self.line1
    }

    pub fn line2(&self) -> Option<&str> {
        self.line2.as_deref()
    }

    pub fn city(&self) -> &str {
        &self.city
    }

    pub fn postal_code(&self) -> &PostalCode {
        &self.postal_code
    }

    pub fn province_country(&self) -> &ProvinceCountry {
        &self.province_country
    }
}
