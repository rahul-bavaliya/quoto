// tests/quote_scenarios.rs: whole-quote scenarios 🧾
//
// These robots use Quoto exactly like a real program would: only through
// the `pub` front door in `src/lib.rs`. They can't see private fields.

use quoto::{
    Address, Contact, Customer, LineItem, LineKind, PhoneNumber, PostalCode, Province, Quote,
    QuoteError,
};

/// A customer who lives in the given province (that's what decides the taxes).
fn customer_in(province: Province) -> Customer {
    let (city, postal_code) = match province {
        Province::Saskatchewan => ("Regina", "S4P 3Y2"),
        Province::Alberta => ("Calgary", "T2P 1J9"),
    };
    let contact = Contact::new(PhoneNumber::parse("306-555-0142").unwrap(), None, None);
    let address = Address::new(
        String::from("123 Main St"),
        None,
        String::from(city),
        PostalCode::parse(postal_code).unwrap(),
        province,
    )
    .unwrap();
    Customer::new(String::from("Jane Smith"), None, None, contact, address).unwrap()
}

/// The standard 5-line oil-change job ($198.00 before tax).
fn oil_change_quote(province: Province) -> Quote {
    let mut quote = Quote::new(customer_in(province));
    let lines = [
        ("Oil Filter", 1.0, 2500, LineKind::Part),
        ("Oil Change Labour", 0.5, 16000, LineKind::Labour),
        ("Engine Oil", 6.0, 850, LineKind::Part),
        ("Shop Supplies", 1.0, 1000, LineKind::Fee),
        ("Air Filter", 1.0, 3200, LineKind::Part),
    ];
    for (description, quantity, price_cents, kind) in lines {
        let item = LineItem::new(String::from(description), quantity, price_cents, kind).unwrap();
        quote.add_item(item).unwrap();
    }
    quote
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
    assert!(text.contains("Quote: Saskatchewan"));
    assert!(text.contains("GST 5%: $9.90"));
    assert!(text.contains("PST 6%: $11.88"));
    assert!(text.ends_with("Total: $219.78"));
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
