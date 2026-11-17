// tests/samples.rs: tests for the samples feature (public API only)

use quoto::samples::*;
use quoto::*;

// ===== samples =====

#[test]
fn every_sample_builds_without_an_error() {
    assert!(customer_in(Province::Saskatchewan).is_ok());
    assert!(customer_in(Province::Alberta).is_ok());
    assert!(prairie_farms().is_ok());
    assert!(northern_oilfield().is_ok());
    assert!(corolla().is_ok());
    assert!(oilfield_truck().is_ok());
}

#[test]
fn the_oil_change_is_198_dollars_before_tax() {
    let quote = oil_change_quote(Province::Saskatchewan).unwrap();
    assert_eq!(quote.subtotal(), 19800);
}

#[test]
fn the_demo_quotes_total_what_the_demo_prints() {
    let first = quote_number();
    let second = first.next().unwrap();
    let sk = demo_quote(
        first,
        quote_time(),
        branch_in(Province::Saskatchewan).unwrap(),
        prairie_farms().unwrap(),
        corolla().unwrap(),
    )
    .unwrap();
    let ab = demo_quote(
        second,
        quote_time(),
        branch_in(Province::Alberta).unwrap(),
        northern_oilfield().unwrap(),
        oilfield_truck().unwrap(),
    )
    .unwrap();
    assert_eq!(sk.total(), 13653);
    assert_eq!(ab.total(), 12915);
}

#[test]
fn a_demo_quote_shows_the_time_it_is_given() {
    let later = quote_time() + chrono::Duration::hours(5);
    let quote = demo_quote(
        quote_number(),
        later,
        branch_in(Province::Saskatchewan).unwrap(),
        prairie_farms().unwrap(),
        corolla().unwrap(),
    )
    .unwrap();
    assert!(quote.to_string().contains("Date: 2026-11-16 14:30:00"));
}

#[test]
fn the_demo_catalog_knows_the_corolla_and_the_tractor() {
    let catalog = equipment_lookup();
    assert!(catalog.lookup(COROLLA_SERIAL).unwrap().is_some());
    assert!(catalog.lookup(TRACTOR_SERIAL).unwrap().is_some());
}

#[test]
fn the_demo_catalog_does_not_know_the_oilfield_truck() {
    let catalog = equipment_lookup();
    assert_eq!(catalog.lookup("1FT8W3BT0NEC12345"), Ok(None));
}

#[test]
fn the_corolla_from_the_catalog_matches_the_one_typed_in() {
    let draft = equipment_lookup().lookup(COROLLA_SERIAL).unwrap().unwrap();
    let from_lookup = corolla_from(draft).unwrap();
    let by_hand = corolla().unwrap();
    assert_eq!(from_lookup.to_string(), by_hand.to_string());
    assert_eq!(
        from_lookup.source(),
        EquipmentSource::Lookup("demo catalog")
    );
    assert_eq!(by_hand.source(), EquipmentSource::Manual);
}
