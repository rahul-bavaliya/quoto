// src/equipment/meter.rs

use super::MeterType;
use crate::errors::QuoteError;
use std::fmt;

/// The reading is optional on a quote: the unit may not be in the shop yet.
/// (A repair order will require it, in Phase 6.)
#[derive(Debug, PartialEq)]
pub struct Meter {
    meter: Option<f64>,
    meter_type: MeterType,
}

impl Meter {
    /// A reading, when there is one, must be a real number that is zero or more.
    pub fn new(meter: Option<f64>, meter_type: MeterType) -> Result<Self, QuoteError> {
        if let Some(reading) = meter
            && (!reading.is_finite() || reading < 0.0)
        {
            return Err(QuoteError::InvalidMeterReading { reading });
        }
        Ok(Meter { meter, meter_type })
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
            Meter::new(Some(100.0), MeterType::Hour).unwrap(),
            Meter::new(Some(100.0), MeterType::Hour).unwrap()
        );
        assert_ne!(
            Meter::new(Some(100.0), MeterType::Hour).unwrap(),
            Meter::new(Some(100.0), MeterType::Mile).unwrap()
        );
    }

    #[test]
    fn a_meter_without_a_reading_is_allowed_on_a_quote() {
        let meter = Meter::new(None, MeterType::Hour).unwrap();
        assert_eq!(meter.to_string(), "not recorded");
    }

    #[test]
    fn a_zero_reading_is_allowed() {
        assert!(Meter::new(Some(0.0), MeterType::Kilometer).is_ok());
    }

    #[test]
    fn a_negative_reading_is_refused() {
        assert_eq!(
            Meter::new(Some(-1.0), MeterType::Kilometer),
            Err(QuoteError::InvalidMeterReading { reading: -1.0 })
        );
    }

    #[test]
    fn a_reading_that_is_not_a_number_is_refused() {
        let result = Meter::new(Some(f64::NAN), MeterType::Hour);
        assert!(matches!(
            result,
            Err(QuoteError::InvalidMeterReading { .. })
        ));
    }
}
