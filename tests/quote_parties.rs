// tests/quote_parties.rs: who a quote is for (customer and machine), quick quotes, accepting

use quoto::*;
use uuid::Uuid;

fn day() -> chrono::NaiveDate {
    samples::quote_time().date()
}

fn branch() -> Branch {
    samples::branch_in(Province::Saskatchewan).unwrap()
}

/// A quick quote: just the branch.
fn quick_quote() -> Quote {
    Quote::new(samples::quote_number(), samples::quote_time(), branch()).unwrap()
}

fn customer() -> Customer {
    samples::customer_in(Province::Saskatchewan).unwrap()
}

fn machine() -> Equipment {
    samples::corolla().unwrap()
}

fn id(number: u128) -> Uuid {
    Uuid::from_u128(number)
}

fn a_line() -> LineItem {
    LineItem::new(String::from("Oil filter"), 1.0, 500, LineKind::Part).unwrap()
}

// ===== a quick quote: only the branch =====

#[test]
fn a_new_quote_has_a_branch_and_nobody_else() {
    let quote = quick_quote();
    assert_eq!(quote.branch(), &branch());
    assert!(quote.customer().is_none());
    assert!(quote.equipment().is_none());
}

#[test]
fn a_quick_quote_can_be_priced_and_taxed_from_the_branch_province() {
    let mut quote = quick_quote();
    quote.add_item(a_line()).unwrap();
    assert_eq!(quote.subtotal(), 500);
    assert_eq!(quote.gst_amount(), 25); // GST 5% of $5.00
    assert_eq!(quote.pst_amount(), 30); // Saskatchewan PST 6% of $5.00
    assert_eq!(quote.total(), 555);
}

#[test]
fn a_quick_quote_prints_that_the_customer_and_machine_are_to_be_added() {
    let text = quick_quote().to_string();
    assert!(text.contains("Branch: Regina Branch\n"));
    assert!(text.contains("Customer: to be added\n"));
    assert!(text.contains("Equipment: to be added\n"));
    assert!(!text.contains("Contact:"));
}

// ===== adding the customer =====

#[test]
fn the_customer_can_be_added_later() {
    let mut quote = quick_quote();
    quote.attach_customer(customer()).unwrap();
    assert_eq!(quote.customer(), Some(&customer()));
    assert!(quote.to_string().contains("Customer: Jane Smith"));
}

#[test]
fn a_second_customer_is_refused_and_the_first_stays() {
    let mut quote = quick_quote();
    quote.attach_customer(customer()).unwrap();
    let other = samples::northern_oilfield().unwrap();
    assert!(matches!(
        quote.attach_customer(other),
        Err(QuoteError::AlreadyAttached { what: "customer" })
    ));
    assert_eq!(quote.customer(), Some(&customer()));
}

// ===== adding the machine =====

#[test]
fn a_machine_needs_a_customer_first() {
    let mut quote = quick_quote();
    assert!(matches!(
        quote.attach_equipment(machine()),
        Err(QuoteError::EquipmentNeedsCustomer)
    ));
    assert!(quote.equipment().is_none());
}

#[test]
fn the_machine_can_be_added_after_the_customer() {
    let mut quote = quick_quote();
    quote.attach_customer(customer()).unwrap();
    quote.attach_equipment(machine()).unwrap();
    assert_eq!(quote.equipment(), Some(&machine()));
}

#[test]
fn a_second_machine_is_refused_and_the_first_stays() {
    let mut quote = quick_quote();
    quote.attach_customer(customer()).unwrap();
    quote.attach_equipment(machine()).unwrap();
    assert!(matches!(
        quote.attach_equipment(samples::oilfield_truck().unwrap()),
        Err(QuoteError::AlreadyAttached { what: "equipment" })
    ));
    assert_eq!(quote.equipment(), Some(&machine()));
}

#[test]
fn a_machine_that_belongs_to_the_customer_is_accepted() {
    let mut quote = quick_quote();
    quote
        .attach_customer(customer().with_id(id(0xAAA)))
        .unwrap();
    quote
        .attach_equipment(machine().with_customer_id(id(0xAAA)))
        .unwrap();
    assert!(quote.equipment().is_some());
}

#[test]
fn a_machine_that_belongs_to_another_customer_is_refused() {
    let mut quote = quick_quote();
    quote
        .attach_customer(customer().with_id(id(0xAAA)))
        .unwrap();
    assert!(matches!(
        quote.attach_equipment(machine().with_customer_id(id(0xBBB))),
        Err(QuoteError::EquipmentNotOwnedByCustomer)
    ));
    assert!(quote.equipment().is_none());
}

#[test]
fn nothing_to_compare_means_the_machine_is_accepted() {
    // Not saved yet: the machine has no owner id, or the customer has no id.
    let mut unsaved = quick_quote();
    unsaved.attach_customer(customer()).unwrap();
    unsaved
        .attach_equipment(machine().with_customer_id(id(0xBBB)))
        .unwrap();

    let mut no_owner = quick_quote();
    no_owner
        .attach_customer(customer().with_id(id(0xAAA)))
        .unwrap();
    no_owner.attach_equipment(machine()).unwrap();
}

// ===== a decided quote is locked =====

#[test]
fn a_rejected_quote_cannot_get_a_customer_or_a_machine() {
    let mut quote = quick_quote();
    quote.reject("Found a lower price elsewhere").unwrap();
    assert!(matches!(
        quote.attach_customer(customer()),
        Err(QuoteError::AlreadyDecided { .. })
    ));
    assert!(quote.customer().is_none());
}

// ===== accepting needs both =====

#[test]
fn a_quote_without_a_customer_cannot_be_accepted() {
    let mut quote = quick_quote();
    assert!(matches!(
        quote.accept(day()),
        Err(QuoteError::QuoteIncomplete {
            missing: "customer"
        })
    ));
    assert_eq!(quote.status(), &QuoteStatus::Pending);
}

#[test]
fn a_quote_without_a_machine_cannot_be_accepted() {
    let mut quote = quick_quote();
    quote.attach_customer(customer()).unwrap();
    assert!(matches!(
        quote.accept(day()),
        Err(QuoteError::QuoteIncomplete {
            missing: "equipment"
        })
    ));
    assert_eq!(quote.status(), &QuoteStatus::Pending);
}

#[test]
fn a_complete_quote_can_be_accepted() {
    let mut quote = quick_quote();
    quote.attach_customer(customer()).unwrap();
    quote.attach_equipment(machine()).unwrap();
    quote.accept(day()).unwrap();
    assert_eq!(quote.status(), &QuoteStatus::Accepted);
}

#[test]
fn a_quick_quote_can_still_be_rejected_with_a_reason() {
    let mut quote = quick_quote();
    quote.reject("Just checking prices").unwrap();
    assert_eq!(quote.status().label(), "rejected");
}

#[test]
fn a_decided_quote_is_reported_as_decided_before_it_is_reported_as_incomplete() {
    let mut quote = quick_quote();
    quote.reject("No thanks").unwrap();
    assert!(matches!(
        quote.accept(day()),
        Err(QuoteError::AlreadyDecided { .. })
    ));
}

#[test]
fn an_incomplete_quote_is_reported_as_incomplete_before_it_is_reported_as_expired() {
    let mut quote = quick_quote();
    let long_after = day() + chrono::Days::new(400);
    assert!(matches!(
        quote.accept(long_after),
        Err(QuoteError::QuoteIncomplete { .. })
    ));
}

#[test]
fn the_error_messages_never_contain_customer_details() {
    let mut quote = quick_quote();
    quote.attach_customer(customer()).unwrap();
    let message = quote.attach_customer(customer()).unwrap_err().to_string();
    assert!(!message.contains("Jane"));
    assert!(!message.contains("555"));
}
