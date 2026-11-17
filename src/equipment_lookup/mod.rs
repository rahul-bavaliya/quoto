// src/equipment_lookup/mod.rs: "tell me the serial number, I tell you what the machine is"

use crate::equipment::EquipmentDraft;
use crate::errors::QuoteResult;

mod chain;
mod in_memory;

pub use chain::ChainedLookup;
pub use in_memory::InMemoryEquipmentLookup;

/// Anything that can look up a machine by its serial number: a catalog in
/// memory, a government service, a maker's catalog. `Quote` never talks to
/// the outside world; the service writer's screen asks a lookup and shows the answer.
///
/// - `Ok(Some(draft))`: found it.
/// - `Ok(None)`: this lookup doesn't know that serial number (not an error).
/// - `Err(_)`: the lookup could not even be asked (service down, bad answer).
///
/// In every case except `Some`, the service writer types the details in by hand.
pub trait EquipmentLookup {
    fn lookup(&self, serial: &str) -> QuoteResult<Option<EquipmentDraft>>;
}

/// Serial numbers are compared without spaces around them and without
/// caring about capital letters ("1ft8w3... " is the same machine as "1FT8W3...").
pub(crate) fn normalize_serial(serial: &str) -> String {
    serial.trim().to_uppercase()
}
