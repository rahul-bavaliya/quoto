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
