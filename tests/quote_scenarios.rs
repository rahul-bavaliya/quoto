// tests/quote_scenarios.rs: whole-quote scenarios 🧾
//
// These robots use Quoto exactly like a real program would: only through
// the `pub` front door in `src/lib.rs`. They can't see private fields.

use quoto::{LineItem, LineKind, Province, Quote, QuoteError, samples};

/// The standard 5-line oil-change job ($198.00 before tax).
fn oil_change_quote(province: Province) -> Quote {
    samples::oil_change_quote(province).unwrap()
}

fn goodwill(cents: i64) -> LineItem {
    LineItem::new(String::from("Goodwill"), 1.0, cents, LineKind::Credit).unwrap()
}

#[test]
fn saskatchewan_oil_change_totals_219_78() {
    let quote = oil_change_quote(Province::Saskatchewan);
    assert_eq!(quote.total(), 21978); // 198.00 + 9.90 GST + 11.88 PST
}

#[test]
fn alberta_oil_change_totals_207_90() {
    let quote = oil_change_quote(Province::Alberta);
    assert_eq!(quote.total(), 20790); // 198.00 + 9.90 GST, no PST
}

#[test]
fn saskatchewan_with_50_goodwill_totals_164_28() {
    let mut quote = oil_change_quote(Province::Saskatchewan);
    quote.add_item(goodwill(5000)).unwrap();
    assert_eq!(quote.total(), 16428); // 148.00 + 7.40 + 8.88
}

#[test]
fn alberta_with_50_goodwill_totals_155_40() {
    let mut quote = oil_change_quote(Province::Alberta);
    quote.add_item(goodwill(5000)).unwrap();
    assert_eq!(quote.total(), 15540); // 148.00 + 7.40
}

#[test]
fn credit_bigger_than_the_job_is_refused() {
    let mut quote = oil_change_quote(Province::Saskatchewan);
    let result = quote.add_item(goodwill(30000)); // $300 credit on a $198 job
    assert_eq!(
        result,
        Err(QuoteError::CreditTooLarge {
            credit_cents: 30000,
            subtotal_cents: 19800,
        })
    );
    assert_eq!(quote.subtotal(), 19800); // the guard kept it OUT of the quote
}

#[test]
fn credit_equal_to_the_job_makes_it_free() {
    let mut quote = oil_change_quote(Province::Alberta);
    assert!(quote.add_item(goodwill(19800)).is_ok());
    assert_eq!(quote.total(), 0);
}

#[test]
fn a_bad_line_never_reaches_the_quote() {
    let result = LineItem::new(String::from("Oil Filter"), 0.0, 2500, LineKind::Part);
    assert!(matches!(result, Err(QuoteError::BadQuantity { .. })));
}

#[test]
fn printed_quote_shows_taxes_and_total() {
    let text = oil_change_quote(Province::Saskatchewan).to_string();
    assert!(text.contains("Quote Q-2026-00000001: Saskatchewan"));
    assert!(text.contains("Date: 2026-11-16 09:30:00"));
    assert!(text.contains("GST 5%: $9.90"));
    assert!(text.contains("PST 6%: $11.88"));
    assert!(text.ends_with("Total: $219.78"));
}

#[test]
fn printed_quote_shows_every_line_and_every_total_in_order() {
    // Worked out by hand: parts 25.00 + 51.00 + 32.00 = 108.00, labour 0.5 x 160.00 = 80.00,
    // fee 10.00, no credits; subtotal 198.00.
    let text = oil_change_quote(Province::Saskatchewan).to_string();
    let wanted = [
        "[PART] Oil Filter: 1 x $25.00 = $25.00",
        "[LABOUR] Oil Change Labour: 0.5 x $160.00 = $80.00",
        "[PART] Engine Oil: 6 x $8.50 = $51.00",
        "[FEE] Shop Supplies: 1 x $10.00 = $10.00",
        "[PART] Air Filter: 1 x $32.00 = $32.00",
        "Labour Total: $80.00",
        "Parts Total: $108.00",
        "Fees Total: $10.00",
        "Credit Total: $0.00",
        "Subtotal: $198.00",
        "GST 5%: $9.90",
        "PST 6%: $11.88",
        "Total: $219.78",
    ];
    let mut from = 0;
    for line in wanted {
        let found = text[from..]
            .find(&format!("\n{line}"))
            .unwrap_or_else(|| panic!("`{line}` is missing or out of order in:\n{text}"));
        from += found + 1;
    }
}

#[test]
fn printed_alberta_quote_has_no_pst_line() {
    let text = oil_change_quote(Province::Alberta).to_string();
    assert!(!text.contains("PST"));
    assert!(text.ends_with("Total: $207.90"));
}

#[test]
fn printed_quote_shows_the_customer() {
    let text = oil_change_quote(Province::Saskatchewan).to_string();
    assert!(text.contains("Customer: Jane Smith"));
}
