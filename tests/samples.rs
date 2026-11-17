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
        prairie_farms().unwrap(),
        corolla().unwrap(),
    )
    .unwrap();
    let ab = demo_quote(
        second,
        quote_time(),
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
        prairie_farms().unwrap(),
        corolla().unwrap(),
    )
    .unwrap();
    assert!(quote.to_string().contains("Date: 2026-11-16 14:30:00"));
}
