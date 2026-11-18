// src/equipment/meter_type.rs

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MeterType {
    Kilometer,
    Mile,
    Hour,
}

impl MeterType {
    /// Reads "km", "mi" or "hr" (any case). Anything else is refused.
    pub fn from_code(code: &str) -> Result<Self, crate::errors::QuoteError> {
        [MeterType::Kilometer, MeterType::Mile, MeterType::Hour]
            .into_iter()
            .find(|kind| kind.code().eq_ignore_ascii_case(code.trim()))
            .ok_or_else(|| crate::errors::QuoteError::InvalidMeterType {
                value: code.to_string(),
            })
    }

    pub fn code(&self) -> &'static str {
        match self {
            MeterType::Kilometer => "km",
            MeterType::Mile => "mi",
            MeterType::Hour => "hr",
        }
    }
}
