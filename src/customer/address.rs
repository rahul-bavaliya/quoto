// src/customer/address.rs: where a customer is (and which province's taxes apply)

use super::PostalCode;
use crate::errors::QuoteError;
use crate::province::Province;
use std::fmt;

/// A mailing address. The province decides which taxes a quote charges.
#[derive(Debug, PartialEq)]
pub struct Address {
    line1: String,
    line2: Option<String>,
    city: String,
    postal_code: PostalCode,
    province: Province,
}

impl Address {
    pub fn new(
        line1: String,
        line2: Option<String>,
        city: String,
        postal_code: PostalCode,
        province: Province,
    ) -> Result<Self, QuoteError> {
        if line1.trim().is_empty() {
            return Err(QuoteError::MissingField {
                field: "address line 1",
            });
        }
        if city.trim().is_empty() {
            return Err(QuoteError::MissingField { field: "city" });
        }
        Ok(Address {
            line1,
            line2,
            city,
            postal_code,
            province,
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

    pub fn province(&self) -> &Province {
        &self.province
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.line1)?;
        if let Some(line2) = &self.line2 {
            write!(f, ", {}", line2)?;
        }
        write!(
            f,
            ", {}, {} {}",
            self.city,
            self.province.code(),
            self.postal_code
        )
    }
}
