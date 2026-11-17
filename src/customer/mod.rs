// src/customer/mod.rs: who the quote is for

use crate::errors::QuoteError;
use crate::province::Province;
use std::fmt;

mod address;
mod contact;
mod email;
mod phone_number;
mod postal_code;

pub use address::Address;
pub use contact::Contact;
pub use email::Email;
pub use phone_number::PhoneNumber;
pub use postal_code::PostalCode;

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

    pub fn contact(&self) -> &Contact {
        &self.contact
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
