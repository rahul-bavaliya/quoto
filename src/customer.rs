// src/customer.rs: who the quote is for
//
// The pieces live in `src/customer/`: the checked value types (`address`, `contact`, `email`,
// `phone_number`, `postal_code`) and `table` (the `customers` table: create, read, update and set
// the status; never delete).

use crate::errors::QuoteError;
use crate::province::Province;
use crate::status::Status;
use std::fmt;
use uuid::Uuid;

mod address;
mod contact;
mod email;
mod phone_number;
mod postal_code;
mod table;

pub use address::Address;
pub use contact::Contact;
pub use email::Email;
pub use phone_number::PhoneNumber;
pub use postal_code::PostalCode;
pub(crate) use table::{CustomerTable, customer_from_copy, get_customer_with_copy};
pub use table::{
    create_customer, get_customer, list_customers, set_customer_status, update_customer,
};

/// A customer: a person, optionally at a company (e.g. "Jane Smith,
/// Service Manager at Prairie Farms Ltd.").
#[derive(Debug, PartialEq)]
pub struct Customer {
    /// The database's id. Empty until this customer is loaded from the database:
    /// a new one never makes its own id.
    id: Option<Uuid>,
    name: String,
    company: Option<String>,
    designation: Option<String>,
    contact: Contact,
    address: Address,
    /// Never deleted: a customer who is finished with is made `Inactive` (old quotes keep
    /// pointing at them).
    status: Status,
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
            id: None,
            name,
            company,
            designation,
            contact,
            address,
            status: Status::default(),
        })
    }

    /// `None` means "not saved yet". The database gives the id; Quoto never makes one.
    pub fn id(&self) -> Option<Uuid> {
        self.id
    }

    /// Attaches the id the database gave. Used when loading from the database.
    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    /// Attaches the status the database holds. Used when loading from the database.
    pub fn with_status(mut self, status: Status) -> Self {
        self.status = status;
        self
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

    pub fn status(&self) -> Status {
        self.status
    }

    /// Only an active customer can be put on a new quote.
    pub fn is_active(&self) -> bool {
        self.status == Status::Active
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
