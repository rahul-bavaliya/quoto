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

pub struct Equipment {
    make: String,
    model: String,
    model_year: i64,
    serial_number: Option<String>,
    meter: f64,
    previous_meter: Option<f64>,
    meter_type: MeterType,
    equipment_type: String,
    plate: String,
}

impl Equipment {
    pub fn new(
        make: String,
        model: String,
        model_year: i64,
        serial_number: Option<String>,
        meter: f64,
        previous_meter: Option<f64>,
        meter_type: MeterType,
        equipment_type: String,
        plate: String,
    ) -> Self {
        Equipment {
            make,
            model,
            model_year,
            serial_number,
            meter,
            previous_meter,
            meter_type,
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
    pub fn model_year(&self) -> i64 {
        self.model_year
    }
    pub fn serial_number(&self) -> &Option<String> {
        &self.serial_number
    }
    pub fn meter(&self) -> &f64 {
        &self.meter
    }
    pub fn previous_meter(&self) -> &Option<f64> {
        &self.previous_meter
    }
    pub fn meter_type(&self) -> &MeterType {
        &self.meter_type
    }
    pub fn equipment_type(&self) -> &String {
        &self.equipment_type
    }
    pub fn plate(&self) -> &str {
        &self.plate
    }
}

/// e.g. "2025 Toyota Corolla Cross (S/N SERIAL_NUMBER_TEST_1, Plate RB97, 28156 km)"
impl fmt::Display for Equipment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, " {} {} ({})", self.make, self.model, self.model_year)?;

        // "S/N SERIAL_NUMBER_TEST_1, " (only if there is a serial)
        if let Some(serial) = &self.serial_number {
            writeln!(f, "S/N: {} ", serial)?;
        }
        writeln!(f, "Meter: {} {}", self.meter, self.meter_type.code())?;

        // "Plate RB97, 28156 km)" (the last line: no ? and no ;)
        writeln!(f, "Plate: {}", self.plate,)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equipment_sample_test() {
        let equipment = Equipment::new(
            String::from("Toyota"),
            String::from("Corolla Cross"),
            2025,
            Some(String::from("SERIAL_NUMBER_TEST_1")),
            28156.0,
            None,
            MeterType::Kilometer,
            String::from("Car Suv AWD"),
            String::from("RB97"),
        );

        assert_eq!(equipment.make(), "Toyota");
    }
}
