// src/branch/location.rs: where a branch is on the map

use crate::errors::{QuoteError, QuoteResult};

/// A point on the map. A latitude without a longitude means nothing, so they live
/// together: a branch has a whole `Location` or none.
#[derive(Debug, PartialEq)]
pub struct Location {
    latitude: f64,
    longitude: f64,
}

impl Location {
    /// Both numbers must be real, the latitude from -90 to 90 and the longitude from -180 to 180.
    pub fn new(latitude: f64, longitude: f64) -> QuoteResult<Self> {
        let latitude_ok = latitude.is_finite() && (-90.0..=90.0).contains(&latitude);
        let longitude_ok = longitude.is_finite() && (-180.0..=180.0).contains(&longitude);
        if !latitude_ok || !longitude_ok {
            return Err(QuoteError::InvalidLocation);
        }
        Ok(Location {
            latitude,
            longitude,
        })
    }

    pub fn latitude(&self) -> f64 {
        self.latitude
    }

    pub fn longitude(&self) -> f64 {
        self.longitude
    }
}
