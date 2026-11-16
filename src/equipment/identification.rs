// src/equipment/identification.rs

use crate::errors::QuoteError;

/// How we recognise one machine: a serial number, a plate, or both.
/// At least one is required, so a unit can never be anonymous.
#[derive(Debug, PartialEq)]
pub struct Identification {
    serial_number: Option<String>,
    plate: Option<String>,
}

impl Identification {
    pub fn new(serial_number: Option<String>, plate: Option<String>) -> Result<Self, QuoteError> {
        // A blank string is the same as nothing
        let serial_number = serial_number.filter(|s| !s.trim().is_empty());
        let plate = plate.filter(|p| !p.trim().is_empty());

        if serial_number.is_none() && plate.is_none() {
            return Err(QuoteError::MissingIdentification);
        }
        Ok(Identification {
            serial_number,
            plate,
        })
    }

    pub fn serial_number(&self) -> Option<&str> {
        self.serial_number.as_deref()
    }

    pub fn plate(&self) -> Option<&str> {
        self.plate.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identification_with_neither_serial_nor_plate_is_refused() {
        assert_eq!(
            Identification::new(None, None),
            Err(QuoteError::MissingIdentification)
        );
    }

    #[test]
    fn identification_with_only_blank_text_is_refused() {
        assert!(Identification::new(Some(String::from("  ")), Some(String::new())).is_err());
    }

    #[test]
    fn identification_with_only_a_serial_is_accepted() {
        assert!(Identification::new(Some(String::from("ABC123")), None).is_ok());
    }

    #[test]
    fn identification_with_only_a_plate_is_accepted() {
        assert!(Identification::new(None, Some(String::from("RB97"))).is_ok());
    }
}
