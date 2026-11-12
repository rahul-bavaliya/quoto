// src/money/tests.rs: unit tests for money (they can see private fields)

use super::*;

#[test]
fn formats_dollars_and_cents() {
    assert_eq!(format_money(21978), "$219.78");
}

#[test]
fn pads_single_digit_cents() {
    assert_eq!(format_money(105), "$1.05");
}

#[test]
fn formats_zero() {
    assert_eq!(format_money(0), "$0.00");
}

#[test]
fn negative_amounts() {
    assert_eq!(format_money(-150), "-$1.50");
}

#[test]
fn tiny_negative_keeps_its_minus() {
    assert_eq!(format_money(-5), "-$0.05");
}
