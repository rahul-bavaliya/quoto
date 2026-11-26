// src/customer/customer.rs: a customer record (individual or business) for quotes and service records.
//
// The pieces live in `src/customer/`: `customer.rs` holds the customer model and `table`
// (create, read, update and set the status; never delete). Shared value types
// such as `Address`, `Contact`, `Email`, and `PhoneNumber` live in `src/common/`.

use crate::common::{Address, Contact, Email, PhoneNumber};
use crate::customer_address::CustomerAddress;
use crate::errors::{AppError, AppResult};
use crate::province_country::ProvinceCountry;
use crate::status::Status;
use chrono::{DateTime, Utc};
use uuid::Uuid;

// Replace with `sea_orm::entity::prelude::DateTimeUtc` if your crate uses SeaORM's alias.
type DateTimeUtc = DateTime<Utc>;

mod table;

pub use table::{
    create_customer, get_customer, list_customers, set_customer_status, update_customer,
};

/// A person or business that can receive quotes and own equipment.
/// **A customer is never deleted**: when it is not in use it becomes inactive.
///
/// Changing a customer means building the new version with `Customer::new(...)` and
/// giving it the old id with `with_id(...)`: the database then updates the row with
/// that id.
#[derive(Debug, PartialEq)]
pub struct Customer {
    /// The database's id. Empty until this customer is loaded from the database:
    /// a new one never makes its own id.
    id: Option<Uuid>,
    is_business: bool,
    /// Required when `is_business` is true; `None` for an individual customer.
    business_name: Option<String>,
    customer_name: String,
    customer_address: CustomerAddress,
    contact: Contact,
    gst_registration_no: Option<String>,
    status: Status,
    created_at: Option<DateTimeUtc>,
    updated_at: Option<DateTimeUtc>,
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value.and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

impl Customer {
    /// Creates an active individual customer with no database id yet.
    pub fn new(
        customer_name: String,
        customer_address: CustomerAddress,
        contact: Contact,
    ) -> AppResult<Self> {
        let trimmed_name = customer_name.trim();
        if trimmed_name.is_empty() {
            return Err(AppError::MissingField {
                field: "Customer Name",
            });
        }

        Ok(Customer {
            id: None,
            is_business: false,
            business_name: None,
            customer_name: trimmed_name.to_string(),
            customer_address,
            contact,
            gst_registration_no: None,
            status: Status::default(),
            created_at: None,
            updated_at: None,
        })
    }

    /// Marks this customer as a business with a required, non-blank business name.
    pub fn as_business(mut self, business_name: String) -> AppResult<Self> {
        let cleaned = clean_optional(Some(business_name)).ok_or(AppError::MissingField {
            field: "business name",
        })?;
        self.is_business = true;
        self.business_name = Some(cleaned);
        Ok(self)
    }

    pub fn with_customer_address(mut self, customer_address: CustomerAddress) -> Self {
        self.customer_address = customer_address;
        self
    }

    pub fn with_contact(mut self, contact: Contact) -> Self {
        self.contact = contact;
        self
    }

    pub fn with_gst_registration_no(mut self, gst_registration_no: Option<String>) -> Self {
        self.gst_registration_no = clean_optional(gst_registration_no);
        self
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

    /// Sets the status. Used when loading from the database (a new customer is active).
    pub fn with_status(mut self, status: Status) -> Self {
        self.status = status;
        self
    }

    /// Attaches database timestamps when loading from a row.
    pub fn with_timestamps(
        mut self,
        created_at: Option<DateTimeUtc>,
        updated_at: Option<DateTimeUtc>,
    ) -> Self {
        self.created_at = created_at;
        self.updated_at = updated_at;
        self
    }

    /// The customer is no longer in use. Kept for history, but cannot start new quotes.
    pub fn deactivate(&mut self) {
        self.status = Status::Inactive;
    }

    /// The customer is in use again.
    pub fn activate(&mut self) {
        self.status = Status::Active;
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn is_active(&self) -> bool {
        self.status == Status::Active
    }

    pub fn is_business(&self) -> bool {
        self.is_business
    }

    pub fn business_name(&self) -> Option<&str> {
        self.business_name.as_deref()
    }

    pub fn customer_name(&self) -> &str {
        &self.customer_name
    }

    pub fn customer_address(&self) -> &CustomerAddress {
        &self.customer_address
    }

    pub fn mailing_address(&self) -> &Address {
        self.customer_address.mailing_address()
    }

    pub fn use_shipping_as_mailing(&self) -> bool {
        self.customer_address.use_shipping_as_mailing()
    }

    pub fn shipping_address(&self) -> &Address {
        self.customer_address.effective_shipping_address()
    }

    pub fn contact(&self) -> &Contact {
        &self.contact
    }

    pub fn phone1(&self) -> &PhoneNumber {
        self.contact.phone1()
    }

    pub fn phone2(&self) -> Option<&PhoneNumber> {
        self.contact.phone2()
    }

    pub fn email(&self) -> &Email {
        self.contact.email()
    }

    pub fn gst_registration_no(&self) -> Option<&str> {
        self.gst_registration_no.as_deref()
    }

    pub fn created_at(&self) -> Option<DateTimeUtc> {
        self.created_at
    }

    pub fn updated_at(&self) -> Option<DateTimeUtc> {
        self.updated_at
    }

    /// The customer's province comes from the effective shipping address (one source of truth).
    pub fn province_country(&self) -> &ProvinceCountry {
        self.shipping_address().province_country()
    }
}
