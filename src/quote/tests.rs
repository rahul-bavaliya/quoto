// src/quote/tests.rs: unit tests for quote (they can see private fields)

use super::*;

// The 5 standard lines. No credits here: credit robots add their own.
fn sample_quote(province: Province) -> Quote {
    let mut quote = Quote::new(province);
    quote
        .add_item(LineItem::new(String::from("Oil Filter"), 1.0, 2500, LineKind::Part).unwrap())
        .unwrap();
    quote
        .add_item(
            LineItem::new(
                String::from("Oil Change Labour"),
                0.5,
                16000,
                LineKind::Labour,
            )
            .unwrap(),
        )
        .unwrap();
    quote
        .add_item(LineItem::new(String::from("Engine Oil"), 6.0, 850, LineKind::Part).unwrap())
        .unwrap();
    quote
        .add_item(LineItem::new(String::from("Shop Supplies"), 1.0, 1000, LineKind::Fee).unwrap())
        .unwrap();
    quote
        .add_item(LineItem::new(String::from("Air Filter"), 1.0, 3200, LineKind::Part).unwrap())
        .unwrap();
    quote
}

// A credit of any size, for the credit robots
fn credit(cents: i64) -> LineItem {
    LineItem::new(String::from("Goodwill"), 1.0, cents, LineKind::Credit).unwrap()
}

#[test]
fn sk_quote_subtotal() {
    let quote = sample_quote(Province::Saskatchewan);
    assert_eq!(quote.subtotal(), 19800);
}

#[test]
fn ab_quote_has_no_pst() {
    let quote = sample_quote(Province::Alberta);
    assert_eq!(quote.pst_amount(), 0);
}

#[test]
fn sk_quote_gst_cents() {
    let quote = sample_quote(Province::Saskatchewan);
    assert_eq!(quote.gst_amount(), 990);
}

#[test]
fn sk_quote_pst() {
    let quote = sample_quote(Province::Saskatchewan);
    assert_eq!(quote.pst_amount(), 1188);
}

#[test]
fn new_quote_is_empty() {
    let quote = Quote::new(Province::Alberta);
    assert!(quote.items.is_empty());
}

#[test]
fn plain_quote_has_no_credits() {
    let quote = sample_quote(Province::Saskatchewan);
    assert_eq!(quote.credit_total(), 0);
}

#[test]
fn credit_comes_off_before_tax() {
    let mut quote = sample_quote(Province::Saskatchewan);
    quote.add_item(credit(5000)).unwrap();
    assert_eq!(quote.credit_total(), -5000);
    assert_eq!(quote.subtotal(), 14800); // 198.00 - 50.00
    assert_eq!(quote.gst_amount(), 740); // 5% of 148.00, not of 198.00
}

#[test]
fn total_for_adds_only_that_kind() {
    let quote = sample_quote(Province::Saskatchewan);
    assert_eq!(quote.total_for(&LineKind::Part), 10800); // 25.00 + 51.00 + 32.00
    assert_eq!(quote.total_for(&LineKind::Labour), 8000);
    assert_eq!(quote.total_for(&LineKind::Fee), 1000);
    assert_eq!(quote.total_for(&LineKind::Credit), 0);
}
