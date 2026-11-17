// src/equipment/equipment_draft.rs: what a lookup found, before the writer completes it

use super::{EquipmentSource, EquipmentType};

/// What a lookup found out about a machine from its serial number: make,
/// model, year and type. It is not an `Equipment` yet. The service writer
/// still adds the meter reading and the plate, and `Equipment::from_draft`
/// checks everything again.
#[derive(Debug, Clone, PartialEq)]
pub struct EquipmentDraft {
    pub(super) make: String,
    pub(super) model: String,
    pub(super) model_year: Option<i64>,
    pub(super) equipment_type: EquipmentType,
    pub(super) source: EquipmentSource,
}

impl EquipmentDraft {
    /// `provider` is the name of the service that answered, e.g. "NHTSA vPIC".
    pub fn new(
        provider: &'static str,
        make: String,
        model: String,
        model_year: Option<i64>,
        equipment_type: EquipmentType,
    ) -> Self {
        EquipmentDraft {
            make,
            model,
            model_year,
            equipment_type,
            source: EquipmentSource::Lookup(provider),
        }
    }
}
