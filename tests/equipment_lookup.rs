// tests/equipment_lookup.rs: tests for the equipment lookup feature (public API only)

use quoto::*;

// ===== helpers =====

fn corolla_draft() -> EquipmentDraft {
    EquipmentDraft::new(
        "test catalog",
        String::from("Toyota"),
        String::from("Corolla Cross"),
        Some(2025),
        EquipmentType::Car,
    )
}

fn tractor_draft() -> EquipmentDraft {
    EquipmentDraft::new(
        "test catalog",
        String::from("John Deere"),
        String::from("8R 410"),
        Some(2023),
        EquipmentType::Tractor,
    )
}

/// A catalog that knows exactly one machine.
fn catalog_with(serial: &str, draft: EquipmentDraft) -> InMemoryEquipmentLookup {
    let mut catalog = InMemoryEquipmentLookup::new();
    catalog.add(serial, draft);
    catalog
}

/// A lookup that is always broken (like a service that is down).
struct BrokenLookup(&'static str);

impl EquipmentLookup for BrokenLookup {
    fn lookup(&self, _serial: &str) -> QuoteResult<Option<EquipmentDraft>> {
        Err(QuoteError::LookupFailed { provider: self.0 })
    }
}

// ===== in-memory catalog =====

#[test]
fn the_catalog_finds_a_machine_it_knows() {
    let catalog = catalog_with("SERIAL-1", corolla_draft());
    assert_eq!(catalog.lookup("SERIAL-1"), Ok(Some(corolla_draft())));
}

#[test]
fn the_catalog_ignores_capitals_and_spaces_around_the_serial() {
    let catalog = catalog_with("Serial-1", corolla_draft());
    assert_eq!(catalog.lookup("  serial-1 "), Ok(Some(corolla_draft())));
}

#[test]
fn the_catalog_says_none_for_a_serial_it_does_not_know() {
    let catalog = catalog_with("SERIAL-1", corolla_draft());
    assert_eq!(catalog.lookup("SERIAL-2"), Ok(None));
}

#[test]
fn the_catalog_says_none_for_a_blank_serial() {
    let catalog = catalog_with("SERIAL-1", corolla_draft());
    assert_eq!(catalog.lookup(""), Ok(None));
    assert_eq!(catalog.lookup("   "), Ok(None));
}

#[test]
fn adding_the_same_serial_again_replaces_the_machine() {
    let mut catalog = catalog_with("SERIAL-1", corolla_draft());
    catalog.add("SERIAL-1", tractor_draft());
    assert_eq!(catalog.lookup("SERIAL-1"), Ok(Some(tractor_draft())));
}

// ===== chained lookups =====

#[test]
fn the_chain_asks_the_next_lookup_when_the_first_does_not_know() {
    let chain = ChainedLookup::new(vec![
        Box::new(catalog_with("ROAD-1", corolla_draft())),
        Box::new(catalog_with("HEAVY-1", tractor_draft())),
    ]);
    assert_eq!(chain.lookup("ROAD-1"), Ok(Some(corolla_draft())));
    assert_eq!(chain.lookup("HEAVY-1"), Ok(Some(tractor_draft())));
}

#[test]
fn the_first_lookup_that_knows_the_machine_wins() {
    let chain = ChainedLookup::new(vec![
        Box::new(catalog_with("SAME-1", corolla_draft())),
        Box::new(catalog_with("SAME-1", tractor_draft())),
    ]);
    assert_eq!(chain.lookup("SAME-1"), Ok(Some(corolla_draft())));
}

#[test]
fn the_chain_says_none_when_nobody_knows_the_machine() {
    let chain = ChainedLookup::new(vec![
        Box::new(catalog_with("ROAD-1", corolla_draft())),
        Box::new(catalog_with("HEAVY-1", tractor_draft())),
    ]);
    assert_eq!(chain.lookup("UNKNOWN"), Ok(None));
}

#[test]
fn an_empty_chain_knows_nothing() {
    let chain = ChainedLookup::new(Vec::new());
    assert_eq!(chain.lookup("ANYTHING"), Ok(None));
}

#[test]
fn the_chain_skips_a_broken_lookup_when_a_later_one_knows_the_machine() {
    let chain = ChainedLookup::new(vec![
        Box::new(BrokenLookup("broken service")),
        Box::new(catalog_with("HEAVY-1", tractor_draft())),
    ]);
    assert_eq!(chain.lookup("HEAVY-1"), Ok(Some(tractor_draft())));
}

#[test]
fn the_chain_reports_the_first_failure_when_nobody_knows_the_machine() {
    let chain = ChainedLookup::new(vec![
        Box::new(catalog_with("ROAD-1", corolla_draft())),
        Box::new(BrokenLookup("first broken service")),
        Box::new(BrokenLookup("second broken service")),
    ]);
    assert_eq!(
        chain.lookup("UNKNOWN"),
        Err(QuoteError::LookupFailed {
            provider: "first broken service"
        })
    );
}
