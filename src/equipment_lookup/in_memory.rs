// src/equipment_lookup/in_memory.rs: a small catalog kept in memory

use super::{EquipmentLookup, normalize_serial};
use crate::equipment::EquipmentDraft;
use crate::errors::QuoteResult;
use std::collections::HashMap;

/// A catalog of machines kept in memory. It holds made-up machines for the
/// demo and the tests, and it is the stand-in for any real service.
pub struct InMemoryEquipmentLookup {
    machines: HashMap<String, EquipmentDraft>,
}

impl InMemoryEquipmentLookup {
    pub fn new() -> Self {
        InMemoryEquipmentLookup {
            machines: HashMap::new(),
        }
    }

    /// Teaches the catalog one machine. Adding the same serial again replaces it.
    pub fn add(&mut self, serial: &str, draft: EquipmentDraft) {
        self.machines.insert(normalize_serial(serial), draft);
    }
}

impl Default for InMemoryEquipmentLookup {
    fn default() -> Self {
        InMemoryEquipmentLookup::new()
    }
}

impl EquipmentLookup for InMemoryEquipmentLookup {
    fn lookup(&self, serial: &str) -> QuoteResult<Option<EquipmentDraft>> {
        Ok(self.machines.get(&normalize_serial(serial)).cloned())
    }
}
