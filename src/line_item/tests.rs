// src/line_item/tests.rs: unit tests for line_item (they can see private fields)

use super::*;

#[test]
fn labour_line_total_is_hours_times_rate() {
    let labour_item: LineItem =
        LineItem::new(String::from("Oil Change Labour"), 2.0, 50, LineKind::Labour).unwrap();
    assert_eq!(labour_item.line_total(), 100);
}

#[test]
fn three_washers_cost_30_cents() {
    let washers: LineItem = LineItem::new(String::from("Washer"), 3.0, 10, LineKind::Part).unwrap();
    assert_eq!(washers.line_total(), 30);
}

#[test]
fn line_total_rounds_half_cent_up() {
    let line_item: LineItem =
        LineItem::new(String::from("Test Item"), 1.5, 833, LineKind::Part).unwrap();
    assert_eq!(line_item.line_total(), 1250);
}

#[test]
fn negative_quantity_is_refused() {
    let result = LineItem::new(String::from("Oil Filter"), -2.0, 2500, LineKind::Part);
    assert!(matches!(result, Err(QuoteError::BadQuantity { .. })));
}

#[test]
fn negative_price_is_refused() {
    let result = LineItem::new(String::from("Oil Filter"), 2.0, -2500, LineKind::Part);
    assert!(matches!(result, Err(QuoteError::NegativePrice { .. })));
}

#[test]
fn zero_price_is_allowed() {
    let result = LineItem::new(String::from("Free Warranty Part"), 2.0, 0, LineKind::Part);
    assert!(result.is_ok());
}

#[test]
fn zero_quantity_is_refused() {
    let result = LineItem::new(String::from("Oil Filter"), 0.0, 2500, LineKind::Part);
    assert!(matches!(result, Err(QuoteError::BadQuantity { .. })));
}

#[test]
fn credit_line_total_is_negative() {
    let credit = LineItem::new(String::from("Goodwill"), 1.0, 5000, LineKind::Credit).unwrap();
    assert_eq!(credit.line_total(), -5000);
}
