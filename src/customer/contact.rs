// src/customer/contact.rs: how to reach a customer

use super::{Email, PhoneNumber};
use std::fmt;

/// How to reach a customer. The main phone is required; the rest is optional.
#[derive(Debug, PartialEq)]
pub struct Contact {
    phone: PhoneNumber,
    alternate_phone: Option<PhoneNumber>,
    email: Option<Email>,
}

impl fmt::Display for Contact {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // The main phone is always there
        write!(f, "{}", self.phone)?;

        // The optional parts print only when they exist
        if let Some(alt) = &self.alternate_phone {
            write!(f, " · Alternate Phone: {}", alt)?;
        }
        if let Some(email) = &self.email {
            write!(f, " · Email: {}", email)?;
        }
        Ok(())
    }
}

impl Contact {
    /// Takes already-checked values, so it can't fail.
    pub fn new(
        phone: PhoneNumber,
        alternate_phone: Option<PhoneNumber>,
        email: Option<Email>,
    ) -> Self {
        Contact {
            phone,
            alternate_phone,
            email,
        }
    }

    /// The main phone (always there).
    pub fn phone(&self) -> &PhoneNumber {
        &self.phone
    }

    pub fn alternate_phone(&self) -> Option<&PhoneNumber> {
        self.alternate_phone.as_ref()
    }

    pub fn email(&self) -> Option<&Email> {
        self.email.as_ref()
    }
}
