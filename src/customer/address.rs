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

#[cfg(test)]
mod tests {
    use super::*;

    fn regina_postal_code() -> PostalCode {
        PostalCode::parse("S4P 3Y2").unwrap()
    }

    #[test]
    fn address_shows_on_one_line() {
        let address = Address::new(
            String::from("123 Main St"),
            Some(String::from("Unit 4")),
            String::from("Regina"),
            regina_postal_code(),
            Province::Saskatchewan,
        )
        .unwrap();
        assert_eq!(
            address.to_string(),
            "123 Main St, Unit 4, Regina, SK S4P 3Y2"
        );
    }

    #[test]
    fn address_without_line1_is_refused() {
        let result = Address::new(
            String::from("  "),
            None,
            String::from("Regina"),
            regina_postal_code(),
            Province::Saskatchewan,
        );
        assert_eq!(
            result,
            Err(QuoteError::MissingField {
                field: "address line 1"
            })
        );
    }

    #[test]
    fn address_without_city_is_refused() {
        let result = Address::new(
            String::from("123 Main St"),
            None,
            String::from(""),
            regina_postal_code(),
            Province::Saskatchewan,
        );
        assert_eq!(result, Err(QuoteError::MissingField { field: "city" }));
    }
}
