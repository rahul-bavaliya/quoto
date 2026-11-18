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
impl Meter {
    /// The reading, if one was taken.
    pub fn reading(&self) -> Option<f64> {
        self.meter
    }

    pub fn meter_type(&self) -> MeterType {
        self.meter_type
    }
}

impl fmt::Display for Meter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.meter {
            Some(reading) => write!(f, "{} {}", reading, self.meter_type.code()),
            None => write!(f, "not recorded"),
        }
    }
}
