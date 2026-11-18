// src/equipment/equipment_type.rs

/// What kind of machine this is. A fixed list, so "Truck" and "truck "
/// can never become two different kinds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EquipmentType {
    Car,
    Truck,
    Tractor,
    Trailer,
    Other,
}

impl EquipmentType {
    /// Reads "car", "truck", "tractor", "trailer" or "other" (any case). Anything else is refused.
    pub fn from_label(label: &str) -> Result<Self, crate::errors::QuoteError> {
        [
            EquipmentType::Car,
            EquipmentType::Truck,
            EquipmentType::Tractor,
            EquipmentType::Trailer,
            EquipmentType::Other,
        ]
        .into_iter()
        .find(|kind| kind.label().eq_ignore_ascii_case(label.trim()))
        .ok_or_else(|| crate::errors::QuoteError::InvalidEquipmentType {
            value: label.to_string(),
        })
    }

    pub fn label(&self) -> &'static str {
        match self {
            EquipmentType::Car => "Car",
            EquipmentType::Truck => "Truck",
            EquipmentType::Tractor => "Tractor",
            EquipmentType::Trailer => "Trailer",
            EquipmentType::Other => "Other",
        }
    }
}
