// src/equipment.rs

use crate::errors::QuoteError;
use crate::status::Status;
use std::fmt;
use uuid::Uuid;

mod equipment_draft;
mod equipment_source;
mod equipment_type;
mod identification;
mod meter;
mod meter_type;
mod table;

pub use equipment_draft::EquipmentDraft;
pub use equipment_source::EquipmentSource;
pub(crate) use equipment_source::{DEMO_CATALOG, NHTSA_VPIC};
pub use equipment_type::EquipmentType;
pub use identification::Identification;
pub use meter::Meter;
pub use meter_type::MeterType;
pub use table::{
    EquipmentFilter, OwnerFilter, create_equipment, get_equipment, list_equipment,
    set_equipment_owner, set_equipment_status, update_equipment,
};
pub(crate) use table::{EquipmentTable, equipment_from_copy, get_equipment_with_copy};

#[derive(Debug, PartialEq)]
pub struct Equipment {
    /// The database's id. Empty until this unit is loaded from the database:
    /// a new one never makes its own id.
    id: Option<Uuid>,
    /// The customer who owns this machine (the database's id for that customer).
    /// Empty until the machine is loaded from the database.
    customer_id: Option<Uuid>,
    make: String,
    model: String,
    model_year: Option<i64>,
    identification: Identification,
    meter: Meter,
    equipment_type: EquipmentType,
    source: EquipmentSource,
    /// Whether the machine is in use. A machine is never erased: "deleting" it archives it
    /// (`Inactive`), and it can be restored.
    status: Status,
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
            id: None,
            customer_id: None,
            make,
            model,
            model_year,
            identification,
            meter,
            equipment_type,
            source: EquipmentSource::Manual,
            status: Status::default(),
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
    /// `None` means "not saved yet". The database gives the id; Quoto never makes one.
    pub fn id(&self) -> Option<Uuid> {
        self.id
    }

    /// Attaches the id the database gave. Used when loading from the database.
    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    /// The owner, once known. A quote uses it to refuse a machine that belongs to someone else.
    pub fn customer_id(&self) -> Option<Uuid> {
        self.customer_id
    }

    /// Says which customer owns this machine. Used when loading from the database.
    pub fn with_customer_id(mut self, customer_id: Uuid) -> Self {
        self.customer_id = Some(customer_id);
        self
    }

    /// Attaches the status the database holds. Used when loading from the database.
    pub fn with_status(mut self, status: Status) -> Self {
        self.status = status;
        self
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

    pub fn meter(&self) -> &Meter {
        &self.meter
    }

    pub fn status(&self) -> Status {
        self.status
    }

    /// Only an active (not archived) machine can be put on a new quote.
    pub fn is_active(&self) -> bool {
        self.status == Status::Active
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
