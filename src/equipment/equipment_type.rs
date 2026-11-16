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
    pub fn label(&self) -> &str {
        match self {
            EquipmentType::Car => "Car",
            EquipmentType::Truck => "Truck",
            EquipmentType::Tractor => "Tractor",
            EquipmentType::Trailer => "Trailer",
            EquipmentType::Other => "Other",
        }
    }
}
