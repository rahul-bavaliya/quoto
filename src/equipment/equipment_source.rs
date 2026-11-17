// src/equipment/equipment_source.rs: where a machine's details came from

use std::fmt;

/// Where the details of a machine came from. We keep it so a record typed by
/// hand is never mistaken for one an outside service supplied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EquipmentSource {
    /// The service writer typed it in.
    Manual,
    /// An outside service or catalog supplied it; the name says which one.
    Lookup(&'static str),
}

impl fmt::Display for EquipmentSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EquipmentSource::Manual => write!(f, "entered by hand"),
            EquipmentSource::Lookup(provider) => write!(f, "looked up with {provider}"),
        }
    }
}
