// src/equipment/meter.rs

use super::MeterType;
use std::fmt;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
