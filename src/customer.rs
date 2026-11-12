// src/customer.rs: who the quote is for

use crate::address::Address;
use crate::contact::Contact;
use crate::errors::QuoteError;
use crate::province::Province;
use std::fmt;

/// A customer: a person, optionally at a company (e.g. "Jane Smith,
/// Service Manager at Prairie Farms Ltd.").
#[derive(Debug, PartialEq)]
pub struct Customer {
    name: String,
    company: Option<String>,
    designation: Option<String>,
    contact: Contact,
    address: Address,
}

impl Customer {
    pub fn new(
        name: String,
        company: Option<String>,
        designation: Option<String>,
        contact: Contact,
        address: Address,
    ) -> Result<Self, QuoteError> {
        if name.trim().is_empty() {
            return Err(QuoteError::MissingField {
                field: "customer name",
            });
        }
        Ok(Customer {
            name,
            company,
            designation,
            contact,
            address,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn company(&self) -> Option<&str> {
        self.company.as_deref()
    }

    pub fn designation(&self) -> Option<&str> {
        self.designation.as_deref()
    }

    pub fn contact(&self) -> &Contact {
        &self.contact
    }

    pub fn address(&self) -> &Address {
        &self.address
    }

    /// The customer's province decides the quote's taxes (one source of truth).
    pub fn province(&self) -> &Province {
        self.address.province()
    }
}

/// e.g. "Jane Smith (Service Manager, Prairie Farms Ltd.)"
impl fmt::Display for Customer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)?;
        match (&self.designation, &self.company) {
            (Some(title), Some(company)) => write!(f, " ({}, {})", title, company),
            (None, Some(company)) => write!(f, " ({})", company),
            (Some(title), None) => write!(f, " ({})", title),
            (None, None) => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::PostalCode;
    use crate::contact::{Email, PhoneNumber};

    fn sample_contact() -> Contact {
        Contact::new(
            PhoneNumber::parse("306-555-0142").unwrap(),
            None,
            Some(Email::parse("jane@prairiefarms.ca").unwrap()),
        )
    }

    fn sample_address() -> Address {
        Address::new(
            String::from("123 Main St"),
            None,
            String::from("Regina"),
            PostalCode::parse("S4P 3Y2").unwrap(),
            Province::Saskatchewan,
        )
        .unwrap()
    }

    #[test]
    fn customer_with_company_and_title_shows_both() {
        let customer = Customer::new(
            String::from("Jane Smith"),
            Some(String::from("Prairie Farms Ltd.")),
            Some(String::from("Service Manager")),
            sample_contact(),
            sample_address(),
        )
        .unwrap();
        assert_eq!(
            customer.to_string(),
            "Jane Smith (Service Manager, Prairie Farms Ltd.)"
        );
    }

    #[test]
    fn customer_without_company_shows_just_the_name() {
        let customer = Customer::new(
            String::from("Bob Lee"),
            None,
            None,
            sample_contact(),
            sample_address(),
        )
        .unwrap();
        assert_eq!(customer.to_string(), "Bob Lee");
    }

    #[test]
    fn customer_without_name_is_refused() {
        let result = Customer::new(
            String::from(""),
            None,
            None,
            sample_contact(),
            sample_address(),
        );
        assert_eq!(
            result,
            Err(QuoteError::MissingField {
                field: "customer name"
            })
        );
    }

    #[test]
    fn customer_province_comes_from_the_address() {
        let customer = Customer::new(
            String::from("Jane Smith"),
            None,
            None,
            sample_contact(),
            sample_address(),
        )
        .unwrap();
        assert_eq!(customer.province(), &Province::Saskatchewan);
    }
}
