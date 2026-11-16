// src/equipment.rs

use crate::errors::QuoteError;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MeterType {
    Kilometer,
    Mile,
    Hour,
}

impl MeterType {
    pub fn code(&self) -> &str {
        match self {
            MeterType::Kilometer => "km",
            MeterType::Mile => "mi",
            MeterType::Hour => "hr",
        }
    }
}

/// What kind of machine this is. A fixed list, so "Truck" and "truck "
/// can never become two different kinds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EquipmentType {
    Car,
    Truck,
    Tractor,
    Trailer,
    Other,
}

impl EquipmentType {
    pub fn label(&self) -> &str {
        match self {
            EquipmentType::Car => "Car",
            EquipmentType::Truck => "Truck",
            EquipmentType::Tractor => "Tractor",
            EquipmentType::Trailer => "Trailer",
            EquipmentType::Other => "Other",
        }
    }
}

/// The reading is optional on a quote: the unit may not be in the shop yet.
/// (A repair order will require it, in Phase 6.)
#[derive(Debug, PartialEq)]
pub struct Meter {
    meter: Option<f64>,
    meter_type: MeterType,
}

impl Meter {
    pub fn new(meter: Option<f64>, meter_type: MeterType) -> Self {
        Meter { meter, meter_type }
    }
}

/// e.g. "28156.98 km", or "not recorded" when there's no reading
impl fmt::Display for Meter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.meter {
            Some(reading) => write!(f, "{} {}", reading, self.meter_type.code()),
            None => write!(f, "not recorded"),
        }
    }
}

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
            return Err(QuoteError::MissingField {
                field: "serial number or plate",
            });
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

    // ---- The identification rule ----

    #[test]
    fn identification_with_neither_serial_nor_plate_is_refused() {
        assert_eq!(
            Identification::new(None, None),
            Err(QuoteError::MissingField {
                field: "serial number or plate"
            })
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

    // ---- The derives: whole values can be compared ----

    #[test]
    fn two_meters_with_the_same_reading_are_equal() {
        assert_eq!(
            Meter::new(Some(100.0), MeterType::Hour),
            Meter::new(Some(100.0), MeterType::Hour)
        );
        assert_ne!(
            Meter::new(Some(100.0), MeterType::Hour),
            Meter::new(Some(100.0), MeterType::Mile)
        );
    }

    #[test]
    fn equipment_type_has_a_label() {
        assert_eq!(EquipmentType::Truck.label(), "Truck");
        assert_eq!(corolla().equipment_type(), EquipmentType::Car);
    }
}
