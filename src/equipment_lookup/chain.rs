// src/equipment_lookup/chain.rs: ask several lookups, one after the other

use super::EquipmentLookup;
use crate::equipment::EquipmentDraft;
use crate::errors::{QuoteError, QuoteResult};

/// Asks its lookups in order, and the first one that knows the machine wins.
/// This is what makes Quoto work for road vehicles and heavy equipment alike:
/// each kind of machine gets its own lookup, and the chain asks them all.
///
/// If nobody knows the serial number but some lookup failed, the first failure
/// is returned, so the person is told why before typing the details in by hand.
pub struct ChainedLookup {
    lookups: Vec<Box<dyn EquipmentLookup>>,
}

impl ChainedLookup {
    pub fn new(lookups: Vec<Box<dyn EquipmentLookup>>) -> Self {
        ChainedLookup { lookups }
    }
}

impl EquipmentLookup for ChainedLookup {
    fn lookup(&self, serial: &str) -> QuoteResult<Option<EquipmentDraft>> {
        let mut first_failure: Option<QuoteError> = None;
        for lookup in &self.lookups {
            match lookup.lookup(serial) {
                Ok(Some(draft)) => return Ok(Some(draft)),
                Ok(None) => {}
                Err(error) => {
                    first_failure.get_or_insert(error);
                }
            }
        }
        match first_failure {
            Some(error) => Err(error),
            None => Ok(None),
        }
    }
}
