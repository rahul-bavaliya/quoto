// src/contact.rs: how to reach a customer (checked phone numbers and email)

use crate::errors::QuoteError;
use std::fmt;

/// A North American phone number. Always stored as exactly 10 digits
/// ("3065550142") and shown as "+1-306-555-0142".
#[derive(Debug, PartialEq)]
pub struct PhoneNumber {
    digits: String,
}

impl PhoneNumber {
    /// Accepts messy input like "(306) 555 0142" or "+1 306 555 0142".
    pub fn parse(input: &str) -> Result<Self, QuoteError> {
        // 1. Clean: keep only the digits
        let mut digits: String = input.chars().filter(|c| c.is_ascii_digit()).collect();

        // 2. "+1 306 555 0142" has 11 digits: drop the country code
        if digits.len() == 11 && digits.starts_with('1') {
            digits.remove(0);
        }

        // 3. Check: exactly 10 digits, and no letters hiding in the input
        let has_letters = input.chars().any(|c| c.is_alphabetic());
        if digits.len() != 10 || has_letters {
            return Err(QuoteError::InvalidPhone {
                input: input.to_string(),
            });
        }
        Ok(PhoneNumber { digits })
    }
}

impl fmt::Display for PhoneNumber {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let d = &self.digits;
        write!(f, "+1-{}-{}-{}", &d[..3], &d[3..6], &d[6..])
    }
}

/// An email address with a sensible shape, stored in lowercase.
/// (A perfect email check is impossible; real systems also send a
/// confirmation email.)
#[derive(Debug, PartialEq)]
pub struct Email {
    value: String,
}

impl Email {
    pub fn parse(input: &str) -> Result<Self, QuoteError> {
        let value = input.trim().to_lowercase();
        let invalid = || QuoteError::InvalidEmail {
            input: input.to_string(),
        };

        if value.contains(char::is_whitespace) {
            return Err(invalid());
        }
        // exactly one '@', with something on both sides
        let (local, domain) = value.split_once('@').ok_or_else(invalid)?;
        if local.is_empty() || domain.contains('@') {
            return Err(invalid());
        }
        // the domain needs a '.' that isn't the first or last character
        let dot_in_middle = domain
            .find('.')
            .is_some_and(|i| i > 0 && !domain.ends_with('.'));
        if !dot_in_middle {
            return Err(invalid());
        }
        Ok(Email { value })
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

/// How to reach a customer. The main phone is required; the rest is optional.
#[derive(Debug, PartialEq)]
pub struct Contact {
    phone: PhoneNumber,
    alternate_phone: Option<PhoneNumber>,
    email: Option<Email>,
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

    // ---- PhoneNumber ----

    #[test]
    fn phone_with_dashes_is_accepted() {
        let phone = PhoneNumber::parse("306-555-0142").unwrap();
        assert_eq!(phone.to_string(), "+1-306-555-0142");
    }

    #[test]
    fn phone_with_brackets_and_spaces_is_cleaned_up() {
        let phone = PhoneNumber::parse("(306) 555 0142").unwrap();
        assert_eq!(phone.to_string(), "+1-306-555-0142");
    }

    #[test]
    fn phone_with_country_code_is_accepted() {
        let phone = PhoneNumber::parse("+1 306 555 0142").unwrap();
        assert_eq!(phone.to_string(), "+1-306-555-0142");
    }

    #[test]
    fn short_phone_is_refused() {
        let result = PhoneNumber::parse("555-0142");
        assert!(matches!(result, Err(QuoteError::InvalidPhone { .. })));
    }

    #[test]
    fn phone_with_letters_is_refused() {
        let result = PhoneNumber::parse("306-555-01AB");
        assert!(matches!(result, Err(QuoteError::InvalidPhone { .. })));
    }

    // ---- Email ----

    #[test]
    fn normal_email_is_accepted() {
        assert!(Email::parse("rahul@example.ca").is_ok());
    }

    #[test]
    fn email_is_stored_in_lowercase() {
        let email = Email::parse("Rahul@Example.CA").unwrap();
        assert_eq!(email.to_string(), "rahul@example.ca");
    }

    #[test]
    fn email_without_at_is_refused() {
        assert!(Email::parse("rahul.example.ca").is_err());
    }

    #[test]
    fn email_without_dot_in_domain_is_refused() {
        assert!(Email::parse("rahul@example").is_err());
    }

    #[test]
    fn email_with_space_is_refused() {
        assert!(Email::parse("ra hul@example.ca").is_err());
    }

    #[test]
    fn email_with_nothing_before_at_is_refused() {
        assert!(Email::parse("@example.ca").is_err());
    }

    #[test]
    fn email_with_two_ats_is_refused() {
        assert!(Email::parse("rahul@@example.ca").is_err());
    }
}
