// src/common/contact.rs

use crate::common::{Email, Language, PhoneNumber};
use crate::errors::AppError;
use std::fmt;

#[derive(Debug, PartialEq)]
pub struct Contact {
    phone1: PhoneNumber,
    phone2: Option<PhoneNumber>,
    email: Email,
    language: Option<Language>,
}

impl Contact {
    /// Builds a `Contact` from validated `PhoneNumber` and `Email` values.
    pub fn new(
        phone1: PhoneNumber,
        phone2: Option<PhoneNumber>,
        email: Email,
        language: Option<Language>,
    ) -> Result<Self, AppError> {
        if phone1.is_empty() {
            return Err(AppError::MissingField { field: "phone1" });
        }
        if email.as_ref().trim().is_empty() {
            return Err(AppError::MissingField { field: "email" });
        }
        Ok(Contact {
            phone1,
            phone2,
            email,
            language,
        })
    }

    pub fn phone1(&self) -> &PhoneNumber {
        &self.phone1
    }

    pub fn phone2(&self) -> Option<&PhoneNumber> {
        self.phone2.as_ref()
    }

    pub fn email(&self) -> &Email {
        &self.email
    }

    pub fn language(&self) -> Option<Language> {
        self.language
    }
}

impl fmt::Display for Contact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.phone1)?;
        if let Some(phone2) = &self.phone2 {
            write!(f, ", alt: {}", phone2)?;
        }
        write!(f, ", {}", self.email)?;
        if let Some(lang) = self.language {
            write!(f, " ({})", lang)?;
        }
        Ok(())
    }
}
