// tests/quote_status.rs: pending / accepted / rejected, and expiry (public API only)

use chrono::{Days, NaiveDate};
use quoto::*;

/// A pending quote with the standard lines, written on the sample day.
fn pending_quote() -> Quote {
    samples::oil_change_quote(Province::Saskatchewan).unwrap()
}

/// The day the sample quote was written.
fn quote_day() -> NaiveDate {
    samples::quote_time().date()
}

/// The last day the sample quote's prices are honoured.
fn last_valid_day() -> NaiveDate {
    pending_quote().expires_on()
}

fn after_expiry() -> NaiveDate {
    last_valid_day() + Days::new(1)
}

fn a_line() -> LineItem {
    LineItem::new(String::from("Extra part"), 1.0, 1000, LineKind::Part).unwrap()
}

// ===== a new quote =====

#[test]
fn a_new_quote_is_pending() {
    assert_eq!(pending_quote().status(), &QuoteStatus::Pending);
}

// ===== expired or not =====

#[test]
fn a_new_quote_is_not_expired_on_the_day_it_is_written() {
    assert!(!pending_quote().is_expired(quote_day()));
}

#[test]
fn the_last_valid_day_is_not_expired_but_the_next_day_is() {
    let quote = pending_quote();
    assert!(!quote.is_expired(last_valid_day()));
    assert!(quote.is_expired(after_expiry()));
}

#[test]
fn only_a_pending_quote_can_expire() {
    let far_future = quote_day() + Days::new(1000);

    let mut accepted = pending_quote();
    accepted.accept(quote_day()).unwrap();
    assert!(!accepted.is_expired(far_future));

    let mut rejected = pending_quote();
    rejected.reject("Too expensive").unwrap();
    assert!(!rejected.is_expired(far_future));
}

#[test]
fn extending_the_expiry_date_makes_an_expired_quote_valid_again() {
    let mut quote = pending_quote();
    assert!(quote.is_expired(after_expiry()));
    quote.set_expiry(after_expiry() + Days::new(7)).unwrap();
    assert!(!quote.is_expired(after_expiry()));
}

// ===== accepting =====

#[test]
fn a_pending_quote_can_be_accepted_on_its_last_valid_day() {
    let mut quote = pending_quote();
    quote.accept(last_valid_day()).unwrap();
    assert_eq!(quote.status(), &QuoteStatus::Accepted);
}

#[test]
fn an_expired_quote_cannot_be_accepted_and_stays_pending() {
    let mut quote = pending_quote();
    assert_eq!(
        quote.accept(after_expiry()),
        Err(QuoteError::QuoteExpired {
            valid_until: last_valid_day()
        })
    );
    assert_eq!(quote.status(), &QuoteStatus::Pending);
}

#[test]
fn an_expired_quote_can_be_accepted_after_the_expiry_date_is_extended() {
    let mut quote = pending_quote();
    quote.set_expiry(after_expiry() + Days::new(7)).unwrap();
    quote.accept(after_expiry()).unwrap();
    assert_eq!(quote.status(), &QuoteStatus::Accepted);
}

// ===== rejecting =====

#[test]
fn a_pending_quote_can_be_rejected_with_a_reason() {
    let mut quote = pending_quote();
    quote.reject("Found a lower price elsewhere").unwrap();
    assert_eq!(
        quote.status(),
        &QuoteStatus::Rejected {
            reason: String::from("Found a lower price elsewhere")
        }
    );
}

#[test]
fn the_spaces_around_a_reason_are_dropped() {
    let mut quote = pending_quote();
    quote.reject("   Too expensive \n").unwrap();
    assert_eq!(
        quote.status(),
        &QuoteStatus::Rejected {
            reason: String::from("Too expensive")
        }
    );
}

#[test]
fn a_rejection_needs_a_reason_and_the_quote_stays_pending() {
    for blank in ["", "   ", "\n\t"] {
        let mut quote = pending_quote();
        assert_eq!(
            quote.reject(blank),
            Err(QuoteError::MissingField {
                field: "rejection reason"
            })
        );
        assert_eq!(quote.status(), &QuoteStatus::Pending);
    }
}

#[test]
fn a_reason_may_be_500_characters_but_not_501() {
    let mut quote = pending_quote();
    assert_eq!(
        quote.reject(&"a".repeat(501)),
        Err(QuoteError::RejectionReasonTooLong { max: 500 })
    );
    assert_eq!(quote.status(), &QuoteStatus::Pending);
    quote.reject(&"a".repeat(500)).unwrap();
}

#[test]
fn the_reason_length_counts_characters_not_bytes() {
    // "é" is one character but two bytes: 500 of them are fine.
    let mut quote = pending_quote();
    quote.reject(&"é".repeat(500)).unwrap();
}

#[test]
fn an_expired_quote_can_still_be_rejected() {
    let mut quote = pending_quote();
    assert!(quote.is_expired(after_expiry()));
    quote.reject("Took too long").unwrap();
    assert_eq!(quote.status().label(), "rejected");
}

// ===== decided quotes are locked =====

#[test]
fn an_accepted_quote_cannot_be_decided_again_or_reopened() {
    let mut quote = pending_quote();
    quote.accept(quote_day()).unwrap();
    let already = Err(QuoteError::AlreadyDecided { status: "accepted" });
    assert_eq!(quote.accept(quote_day()), already);
    assert_eq!(quote.reject("Changed my mind"), already);
    assert_eq!(quote.reopen(), Err(QuoteError::NotRejected));
    assert_eq!(quote.status(), &QuoteStatus::Accepted);
}

#[test]
fn a_rejected_quote_cannot_be_accepted_or_rejected_again() {
    let mut quote = pending_quote();
    quote.reject("Too expensive").unwrap();
    let already = Err(QuoteError::AlreadyDecided { status: "rejected" });
    assert_eq!(quote.accept(quote_day()), already);
    assert_eq!(quote.reject("A different reason"), already);
    assert_eq!(quote.status().to_string(), "rejected (Too expensive)");
}

#[test]
fn a_pending_quote_cannot_be_reopened() {
    assert_eq!(pending_quote().reopen(), Err(QuoteError::NotRejected));
}

#[test]
fn lines_cannot_be_added_once_the_customer_has_decided() {
    let mut accepted = pending_quote();
    accepted.accept(quote_day()).unwrap();
    let total = accepted.total();
    assert_eq!(
        accepted.add_item(a_line()),
        Err(QuoteError::AlreadyDecided { status: "accepted" })
    );
    assert_eq!(accepted.total(), total);

    let mut rejected = pending_quote();
    rejected.reject("Too expensive").unwrap();
    assert_eq!(
        rejected.add_item(a_line()),
        Err(QuoteError::AlreadyDecided { status: "rejected" })
    );
}

#[test]
fn the_promise_date_can_still_be_set_after_acceptance() {
    // The price is locked, the schedule is not: parts arrive, dates move.
    let mut quote = pending_quote();
    quote.accept(quote_day()).unwrap();
    quote.set_promise_date(quote_day() + Days::new(4)).unwrap();
    assert_eq!(quote.promise_date(), Some(quote_day() + Days::new(4)));
}

// ===== reopening =====

#[test]
fn a_rejected_quote_can_be_reopened_and_the_old_reason_is_gone() {
    let mut quote = pending_quote();
    quote.reject("Too expensive").unwrap();
    quote.reopen().unwrap();
    assert_eq!(quote.status(), &QuoteStatus::Pending);
    assert!(!quote.to_string().contains("Too expensive"));
}

#[test]
fn a_reopened_quote_can_be_changed_and_accepted() {
    let mut quote = pending_quote();
    quote.reject("Too expensive").unwrap();
    quote.reopen().unwrap();
    let before = quote.subtotal();
    quote.add_item(a_line()).unwrap();
    assert_eq!(quote.subtotal(), before + 1000);
    quote.accept(quote_day()).unwrap();
    assert_eq!(quote.status(), &QuoteStatus::Accepted);
}

#[test]
fn a_reopened_quote_that_has_expired_still_needs_a_new_expiry_date() {
    let mut quote = pending_quote();
    quote.reject("Too expensive").unwrap();
    quote.reopen().unwrap();
    assert_eq!(
        quote.accept(after_expiry()),
        Err(QuoteError::QuoteExpired {
            valid_until: last_valid_day()
        })
    );
}

// ===== printing =====

#[test]
fn the_printed_quote_shows_the_status() {
    let mut quote = pending_quote();
    assert!(quote.to_string().contains("Status: pending\n"));

    quote.accept(quote_day()).unwrap();
    assert!(quote.to_string().contains("Status: accepted\n"));

    let mut rejected = pending_quote();
    rejected.reject("Found a lower price elsewhere").unwrap();
    assert!(
        rejected
            .to_string()
            .contains("Status: rejected (Found a lower price elsewhere)\n")
    );
}

#[test]
fn the_status_line_comes_right_after_the_promise_date() {
    let text = pending_quote().to_string();
    let promise = text.find("Promise date:").unwrap();
    let status = text.find("Status:").unwrap();
    let customer = text.find("Customer:").unwrap();
    assert!(promise < status && status < customer);
}

#[test]
fn a_status_label_never_contains_the_reason() {
    let mut quote = pending_quote();
    quote.reject("Customer's private reason").unwrap();
    assert_eq!(quote.status().label(), "rejected");
}
