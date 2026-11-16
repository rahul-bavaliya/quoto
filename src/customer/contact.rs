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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contact_with_only_a_phone_prints_just_the_phone() {
        let phone = PhoneNumber::parse("306-555-0142").unwrap();
        let contact = Contact::new(phone, None, None);
        assert_eq!(contact.to_string(), "+1-306-555-0142");
    }

    #[test]
    fn contact_with_phone_and_email_prints_both_on_one_line() {
        let phone = PhoneNumber::parse("306-555-0142").unwrap();
        let email = Email::parse("Rahul@test.com").unwrap();
        let contact = Contact::new(phone, None, Some(email));
        assert_eq!(
            contact.to_string(),
            "+1-306-555-0142 · Email: rahul@test.com"
        );
    }

    #[test]
    fn contact_with_everything_prints_phone_alternate_and_email() {
        let phone = PhoneNumber::parse("306-555-0142").unwrap();
        let alternate_phone = PhoneNumber::parse("306-555-1234").unwrap();
        let email = Email::parse("Rahul@test.com").unwrap();
        let contact = Contact::new(phone, Some(alternate_phone), Some(email));
        assert_eq!(
            contact.to_string(),
            "+1-306-555-0142 · Alternate Phone: +1-306-555-1234 · Email: rahul@test.com"
        );
    }
}
