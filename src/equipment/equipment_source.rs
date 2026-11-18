// src/equipment/equipment_source.rs: where a machine's details came from

use crate::errors::{QuoteError, QuoteResult};
use std::fmt;

/// The names of the services that can supply a machine's details. One list, used by the lookups
/// when they answer and by the database when it reads a machine back (`from_stored`).
pub(crate) const NHTSA_VPIC: &str = "NHTSA vPIC";
pub(crate) const DEMO_CATALOG: &str = "demo catalog";
const KNOWN_PROVIDERS: [&str; 2] = [NHTSA_VPIC, DEMO_CATALOG];

/// Where the details of a machine came from. We keep it so a record typed by
/// hand is never mistaken for one an outside service supplied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EquipmentSource {
    /// The service writer typed it in.
    Manual,
    /// An outside service or catalog supplied it; the name says which one.
    Lookup(&'static str),
}

impl EquipmentSource {
    /// The text kept in the database: "manual", or "lookup:" and the provider's name.
    pub(crate) fn stored(&self) -> String {
        match self {
            EquipmentSource::Manual => String::from("manual"),
            EquipmentSource::Lookup(provider) => format!("lookup:{provider}"),
        }
    }

    /// Reads the text kept in the database. A provider this version doesn't know is refused,
    /// because the name must stay a permanent string.
    pub(crate) fn from_stored(text: &str) -> QuoteResult<Self> {
        if text == "manual" {
            return Ok(EquipmentSource::Manual);
        }
        let provider = text.strip_prefix("lookup:").unwrap_or_default();
        KNOWN_PROVIDERS
            .iter()
            .find(|known| **known == provider)
            .map(|known| EquipmentSource::Lookup(known))
            .ok_or(QuoteError::InvalidEquipmentSource)
    }
}

impl fmt::Display for EquipmentSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EquipmentSource::Manual => write!(f, "entered by hand"),
            EquipmentSource::Lookup(provider) => write!(f, "looked up with {provider}"),
        }
    }
}
