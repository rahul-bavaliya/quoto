// src/equipment.rs

use crate::errors::QuoteError;
use std::fmt;

mod equipment_draft;
mod equipment_source;
mod equipment_type;
mod identification;
mod meter;
mod meter_type;

pub use equipment_draft::EquipmentDraft;
pub use equipment_source::EquipmentSource;
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
    source: EquipmentSource,
}

impl Equipment {
    /// Equipment typed in by the service writer.
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
            source: EquipmentSource::Manual,
        })
    }

    /// Equipment built from a lookup's answer plus what only the writer knows:
    /// the identification (serial and plate) and the meter reading.
    pub fn from_draft(
        draft: EquipmentDraft,
        identification: Identification,
        meter: Meter,
    ) -> Result<Self, QuoteError> {
        let mut equipment = Equipment::new(
            draft.make,
            draft.model,
            draft.model_year,
            identification,
            meter,
            draft.equipment_type,
        )?;
        equipment.source = draft.source;
        Ok(equipment)
    }
    pub fn equipment_type(&self) -> EquipmentType {
        self.equipment_type
    }

    pub fn source(&self) -> EquipmentSource {
        self.source
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
        if let Some(serial) = self.identification.serial_number() {
            write!(f, "\nS/N: {}", serial)?;
        }
        write!(f, "\nMeter: {}", self.meter)?;
        if let Some(plate) = self.identification.plate() {
            write!(f, "\nPlate: {}", plate)?;
        }
        Ok(())
    }
}
