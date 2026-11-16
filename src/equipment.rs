// src/equipment.rs

use crate::errors::QuoteError;
use std::fmt;

mod equipment_type;
mod identification;
mod meter;
mod meter_type;

pub use equipment_type::EquipmentType;
pub use identification::Identification;
pub use meter::Meter;
pub use meter_type::MeterType;

#[derive(Debug, PartialEq)]
pub struct Equipment {
    make: String,
    model: String,
    model_year: Option<i64>,
    identification: Identification,
    meter: Meter,
    equipment_type: EquipmentType,
}

impl Equipment {
    pub fn new(
        make: String,
        model: String,
        model_year: Option<i64>,
        identification: Identification,
        meter: Meter,
        equipment_type: EquipmentType,
    ) -> Result<Self, QuoteError> {
        if make.trim().is_empty() {
            return Err(QuoteError::MissingField { field: "make" });
        }
        if model.trim().is_empty() {
            return Err(QuoteError::MissingField { field: "model" });
        }
        Ok(Equipment {
            make,
            model,
            model_year,
            identification,
            meter,
            equipment_type,
        })
    }
    pub fn make(&self) -> &str {
        &self.make
    }
    pub fn model(&self) -> &str {
        &self.model
    }
    pub fn model_year(&self) -> Option<i64> {
        self.model_year
    }
    pub fn identification(&self) -> &Identification {
        &self.identification
    }
    pub fn serial_number(&self) -> Option<&str> {
        self.identification.serial_number()
    }
    pub fn plate(&self) -> Option<&str> {
        self.identification.plate()
    }
    pub fn meter(&self) -> &Meter {
        &self.meter
    }
    pub fn equipment_type(&self) -> EquipmentType {
        self.equipment_type
    }
}

/// e.g.
/// Toyota Corolla Cross (2025)
/// S/N: SERIAL_NUMBER_TEST_1
/// Meter: 28156.98 km
/// Plate: RB97
impl fmt::Display for Equipment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.make, self.model)?;
        if let Some(year) = self.model_year {
            write!(f, " ({})", year)?;
        }
        // Each later line starts with its own newline, so the last line
        // ends cleanly and the caller decides what comes after it.
        if let Some(serial) = self.serial_number() {
            write!(f, "\nS/N: {}", serial)?;
        }
        write!(f, "\nMeter: {}", self.meter)?;
        if let Some(plate) = self.plate() {
            write!(f, "\nPlate: {}", plate)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corolla() -> Equipment {
        Equipment::new(
            String::from("Toyota"),
            String::from("Corolla Cross"),
            Some(2025),
            Identification::new(
                Some(String::from("SERIAL_NUMBER_TEST_1")),
                Some(String::from("RB97")),
            )
            .unwrap(),
            Meter::new(Some(28156.98), MeterType::Kilometer),
            EquipmentType::Car,
        )
        .unwrap()
    }

    #[test]
    fn equipment_sample_test() {
        assert_eq!(corolla().make(), "Toyota");
    }

    #[test]
    fn equipment_shows_all_its_details() {
        assert_eq!(
            corolla().to_string(),
            "Toyota Corolla Cross (2025)\nS/N: SERIAL_NUMBER_TEST_1\nMeter: 28156.98 km\nPlate: RB97"
        );
    }

    #[test]
    fn equipment_without_optional_details_skips_them() {
        let tractor = Equipment::new(
            String::from("John Deere"),
            String::from("8R 410"),
            None,
            Identification::new(Some(String::from("1RW8R410ABC")), None).unwrap(),
            Meter::new(None, MeterType::Hour),
            EquipmentType::Tractor,
        )
        .unwrap();
        assert_eq!(
            tractor.to_string(),
            "John Deere 8R 410\nS/N: 1RW8R410ABC\nMeter: not recorded"
        );
    }

    // ---- Equipment::new refuses blanks ----

    #[test]
    fn equipment_with_a_blank_make_is_refused() {
        let result = Equipment::new(
            String::from("  "),
            String::from("Corolla"),
            None,
            Identification::new(None, Some(String::from("RB97"))).unwrap(),
            Meter::new(None, MeterType::Kilometer),
            EquipmentType::Car,
        );
        assert_eq!(result, Err(QuoteError::MissingField { field: "make" }));
    }

    #[test]
    fn equipment_with_a_blank_model_is_refused() {
        let result = Equipment::new(
            String::from("Toyota"),
            String::new(),
            None,
            Identification::new(None, Some(String::from("RB97"))).unwrap(),
            Meter::new(None, MeterType::Kilometer),
            EquipmentType::Car,
        );
        assert_eq!(result, Err(QuoteError::MissingField { field: "model" }));
    }

    #[test]
    fn equipment_type_has_a_label() {
        assert_eq!(EquipmentType::Truck.label(), "Truck");
        assert_eq!(corolla().equipment_type(), EquipmentType::Car);
    }
}
