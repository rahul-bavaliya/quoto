// tests/province.rs: tests for the province feature (public API only)

use quoto::*;

// ===== province =====

#[test]
fn saskatchewan_has_correct_pst_rate() {
    let saskatchewan = Province::Saskatchewan;
    assert_eq!(saskatchewan.pst_rate(), Some(6.0));
}

#[test]
fn alberta_has_correct_pst_rate() {
    let alberta = Province::Alberta;
    assert_eq!(alberta.pst_rate(), None);
}

#[test]
fn alberta_has_correct_gst_rate() {
    let alberta = Province::Alberta;
    assert_eq!(alberta.gst_rate(), 5.0);
}
