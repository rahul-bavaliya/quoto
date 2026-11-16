// src/equipment.rs

use std::fmt;

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

pub struct Equipment {
    make: String,
    model: String,
    model_year: Option<i64>,
    serial_number: Option<String>,
    meter: Meter,
    previous_meter: Option<Meter>,
    equipment_type: String,
    plate: Option<String>,
}

impl Equipment {
    // 8 inputs is over clippy's limit of 7. Allowed for now; a builder
    // (or grouping fields into smaller structs) would be the long-term fix.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        make: String,
        model: String,
        model_year: Option<i64>,
        serial_number: Option<String>,
        meter: Meter,
        previous_meter: Option<Meter>,
        equipment_type: String,
        plate: Option<String>,
    ) -> Self {
        Equipment {
            make,
            model,
            model_year,
            serial_number,
            meter,
            previous_meter,
            equipment_type,
            plate,
        }
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
    pub fn serial_number(&self) -> Option<&str> {
        self.serial_number.as_deref()
    }
    pub fn meter(&self) -> &Meter {
        &self.meter
    }
    pub fn previous_meter(&self) -> Option<&Meter> {
        self.previous_meter.as_ref()
    }
    pub fn equipment_type(&self) -> &str {
        &self.equipment_type
    }
    pub fn plate(&self) -> Option<&str> {
        self.plate.as_deref()
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
        writeln!(f)?;

        if let Some(serial) = &self.serial_number {
            writeln!(f, "S/N: {}", serial)?;
        }
        writeln!(f, "Meter: {}", self.meter)?;
        if let Some(plate) = &self.plate {
            writeln!(f, "Plate: {}", plate)?;
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
            Some(String::from("SERIAL_NUMBER_TEST_1")),
            Meter::new(Some(28156.98), MeterType::Kilometer),
            None,
            String::from("Car Suv AWD"),
            Some(String::from("RB97")),
        )
    }

    #[test]
    fn equipment_sample_test() {
        assert_eq!(corolla().make(), "Toyota");
    }

    #[test]
    fn equipment_shows_all_its_details() {
        assert_eq!(
            corolla().to_string(),
            "Toyota Corolla Cross (2025)\nS/N: SERIAL_NUMBER_TEST_1\nMeter: 28156.98 km\nPlate: RB97\n"
        );
    }

    #[test]
    fn equipment_without_optional_details_skips_them() {
        let tractor = Equipment::new(
            String::from("John Deere"),
            String::from("8R 410"),
            None,
            None,
            Meter::new(None, MeterType::Hour),
            None,
            String::from("Tractor"),
            None,
        );
        assert_eq!(
            tractor.to_string(),
            "John Deere 8R 410\nMeter: not recorded\n"
        );
    }
}
