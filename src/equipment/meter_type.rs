// src/equipment/meter_type.rs

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
