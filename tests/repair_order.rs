// tests/repair_order.rs: opening a repair order from an accepted quote, the status flow, the
// due date, the number (public API only; the database side is in tests/database_repair_orders.rs)

use chrono::{Days, NaiveDate, NaiveDateTime};
use quoto::*;
use uuid::Uuid;

fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn day(year: i32, month: u32, date: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, date).unwrap()
}

fn at(date: NaiveDate, hour: u32) -> NaiveDateTime {
    date.and_hms_opt(hour, 0, 0).unwrap()
}

/// The sample quote: saved (it has an id), with a customer and the Corolla (28,156.98 km).
fn saved_quote() -> Quote {
    samples::oil_change_quote(Province::Saskatchewan)
        .unwrap()
        .with_id(id(0xA1))
}

fn accepted_quote() -> Quote {
    let mut quote = saved_quote();
    quote.accept(samples::quote_time().date()).unwrap();
    quote
}

/// An accepted quote whose machine has no meter reading yet.
fn accepted_quote_without_reading() -> Quote {
    let machine = Equipment::new(
        String::from("John Deere"),
        String::from("8R 410"),
        Some(2023),
        Identification::new(Some(String::from("DEMO-8R410-0002")), None).unwrap(),
        Meter::new(None, MeterType::Hour).unwrap(),
        EquipmentType::Tractor,
    )
    .unwrap();
    let mut quote = Quote::new(
        samples::quote_number(),
        samples::quote_time(),
        samples::branch_in(Province::Saskatchewan).unwrap(),
    )
    .unwrap();
    quote
        .attach_customer(samples::customer_in(Province::Saskatchewan).unwrap())
        .unwrap();
    quote.attach_equipment(machine).unwrap();
    quote
        .add_item(LineItem::new(String::from("Service"), 1.0, 5000, LineKind::Labour).unwrap())
        .unwrap();
    let mut quote = quote.with_id(id(0xA2));
    quote.accept(samples::quote_time().date()).unwrap();
    quote
}

fn number() -> RepairOrderNumber {
    RepairOrderNumber::new(2026, 1).unwrap()
}

fn opened_on() -> NaiveDateTime {
    at(day(2026, 11, 17), 8)
}

fn open_order() -> RepairOrder {
    RepairOrder::open(number(), opened_on(), accepted_quote(), None).unwrap()
}

// ===== the number =====

#[test]
fn a_number_prints_as_ro_year_and_eight_digits_and_reads_back() {
    assert_eq!(number().to_string(), "RO-2026-00000001");
    assert_eq!(
        RepairOrderNumber::parse("RO-2026-00000042").unwrap(),
        RepairOrderNumber::new(2026, 42).unwrap()
    );
    let last = RepairOrderNumber::new(9999, 99_999_999).unwrap();
    assert_eq!(last.to_string(), "RO-9999-99999999");
    assert_eq!((last.year(), last.counter()), (9999, 99_999_999));
}

#[test]
fn numbers_that_are_not_repair_order_numbers_are_refused() {
    for bad in [
        "",
        "RO",
        "Q-2026-00000001",
        "RO-2026-1",
        "RO-26-00000001",
        "RO-2026-000000001",
        "RO-2026-0000000a",
        "RO-+026-00000001",
        "ro-2026-00000001",
        "RO-2026-00000001-1",
        "RO-2026-00000000",
        "RO-0999-00000001",
        " RO-2026-00000001",
    ] {
        assert_eq!(
            RepairOrderNumber::parse(bad).unwrap_err().code(),
            "INVALID_REPAIR_ORDER_NUMBER",
            "{bad:?}"
        );
    }
    assert!(RepairOrderNumber::new(2026, 0).is_err());
    assert!(RepairOrderNumber::new(2026, 100_000_000).is_err());
    assert!(RepairOrderNumber::new(999, 1).is_err());
}

#[test]
fn a_number_error_never_repeats_what_was_sent() {
    let error = RepairOrderNumber::parse("RO-ZZmarker9137").unwrap_err();
    assert!(!error.to_string().contains("ZZmarker9137"));
}

// ===== opening =====

#[test]
fn an_accepted_quote_becomes_an_open_repair_order_with_its_own_facts() {
    let order = open_order();
    assert_eq!(order.status(), &RepairOrderStatus::Open);
    assert_eq!(order.number().to_string(), "RO-2026-00000001");
    assert_eq!(order.created_at(), opened_on());
    assert_eq!(order.id(), None); // the database makes the id
    assert_eq!(order.version(), 0);
    assert_eq!(order.started_at(), None);
    assert_eq!(order.closed_at(), None);
    // the work is the quote's: same lines, same total
    assert_eq!(order.quote().total(), 21978);
    assert_eq!(order.quote().items().len(), 5);
}

#[test]
fn the_meter_comes_from_the_quote_unless_the_writer_reads_it_again() {
    let from_quote = RepairOrder::open(number(), opened_on(), accepted_quote(), None).unwrap();
    assert_eq!(from_quote.meter_reading(), 28156.98);
    assert_eq!(from_quote.meter_type(), MeterType::Kilometer);
    let today = RepairOrder::open(number(), opened_on(), accepted_quote(), Some(28160.5)).unwrap();
    assert_eq!(today.meter_reading(), 28160.5);
    // zero is a real reading (a new machine)
    let zero = RepairOrder::open(number(), opened_on(), accepted_quote(), Some(0.0)).unwrap();
    assert_eq!(zero.meter_reading(), 0.0);
}

#[test]
fn the_meter_reading_is_required() {
    let error = RepairOrder::open(
        number(),
        opened_on(),
        accepted_quote_without_reading(),
        None,
    )
    .map(|_| ())
    .unwrap_err();
    assert_eq!(error.code(), "METER_READING_REQUIRED");
    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    // giving it fixes it, in the machine's own unit
    let order = RepairOrder::open(
        number(),
        opened_on(),
        accepted_quote_without_reading(),
        Some(1250.5),
    )
    .unwrap();
    assert_eq!(order.meter_reading(), 1250.5);
    assert_eq!(order.meter_type(), MeterType::Hour);
}

#[test]
fn a_meter_reading_must_be_a_real_number_that_is_zero_or_more() {
    for bad in [-1.0, -0.001, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let error = RepairOrder::open(number(), opened_on(), accepted_quote(), Some(bad))
            .map(|_| ())
            .unwrap_err();
        assert_eq!(error.code(), "INVALID_METER_READING", "{bad}");
    }
}

#[test]
fn only_an_accepted_quote_can_become_a_repair_order() {
    let pending = saved_quote();
    let mut rejected = saved_quote();
    rejected.reject("too dear").unwrap();
    for quote in [pending, rejected] {
        let error = RepairOrder::open(number(), opened_on(), quote, None)
            .map(|_| ())
            .unwrap_err();
        assert_eq!(error.code(), "QUOTE_NOT_ACCEPTED");
        assert_eq!(error.kind(), ErrorKind::BusinessRule);
    }
}

#[test]
fn a_quote_that_was_never_saved_can_not_become_a_repair_order() {
    let mut quote = samples::oil_change_quote(Province::Saskatchewan).unwrap();
    quote.accept(samples::quote_time().date()).unwrap();
    let error = RepairOrder::open(number(), opened_on(), quote, None)
        .map(|_| ())
        .unwrap_err();
    assert_eq!(error.code(), "QUOTE_NOT_SAVED");
}

#[test]
fn the_due_date_starts_as_the_quotes_promise_date() {
    let none = RepairOrder::open(number(), opened_on(), accepted_quote(), None).unwrap();
    assert_eq!(none.due_date(), None); // "to be confirmed"
    let mut quote = saved_quote();
    quote.set_promise_date(day(2026, 11, 20)).unwrap();
    quote.accept(samples::quote_time().date()).unwrap();
    let order = RepairOrder::open(number(), opened_on(), quote, None).unwrap();
    assert_eq!(order.due_date(), Some(day(2026, 11, 20)));
}

// ===== the flow =====

#[test]
fn the_flow_goes_open_then_in_progress_then_completed() {
    let mut order = open_order();
    let start = at(day(2026, 11, 17), 9);
    let end = at(day(2026, 11, 18), 16);
    order.start(start).unwrap();
    assert_eq!(order.status(), &RepairOrderStatus::InProgress);
    assert_eq!(order.started_at(), Some(start));
    assert_eq!(order.closed_at(), None);
    order.complete(end).unwrap();
    assert_eq!(order.status(), &RepairOrderStatus::Completed);
    assert_eq!(order.started_at(), Some(start));
    assert_eq!(order.closed_at(), Some(end));
}

#[test]
fn every_step_is_refused_out_of_turn_and_changes_nothing() {
    let when = at(day(2026, 11, 17), 9);
    // an open one can't be completed
    let mut order = open_order();
    assert_eq!(
        order.complete(when).unwrap_err().code(),
        "REPAIR_ORDER_NOT_IN_PROGRESS"
    );
    assert_eq!(order.status(), &RepairOrderStatus::Open);
    assert_eq!(order.closed_at(), None);
    // one in progress can't be started again (the start time stays)
    order.start(when).unwrap();
    let later = at(day(2026, 11, 17), 11);
    assert_eq!(
        order.start(later).unwrap_err().code(),
        "REPAIR_ORDER_NOT_OPEN"
    );
    assert_eq!(order.started_at(), Some(when));
    // a completed one is final
    order.complete(later).unwrap();
    assert_eq!(
        order.start(later).unwrap_err().code(),
        "REPAIR_ORDER_NOT_OPEN"
    );
    assert_eq!(
        order.complete(later).unwrap_err().code(),
        "REPAIR_ORDER_NOT_IN_PROGRESS"
    );
    assert_eq!(
        order.cancel("too late", later).unwrap_err().code(),
        "REPAIR_ORDER_CLOSED"
    );
    assert_eq!(order.status(), &RepairOrderStatus::Completed);
    for error in [
        order.set_due_date(Some(day(2026, 12, 1))).unwrap_err(),
        order.set_due_date(None).unwrap_err(),
    ] {
        assert_eq!(error.code(), "REPAIR_ORDER_CLOSED");
    }
}

#[test]
fn an_open_or_in_progress_repair_order_can_be_cancelled_with_a_reason() {
    let when = at(day(2026, 11, 17), 9);
    let mut open = open_order();
    open.cancel("  customer sold the machine  ", when).unwrap();
    assert_eq!(
        open.status(),
        &RepairOrderStatus::Cancelled {
            reason: String::from("customer sold the machine")
        }
    );
    assert_eq!(open.closed_at(), Some(when));
    assert_eq!(open.started_at(), None);

    let mut working = open_order();
    working.start(when).unwrap();
    working
        .cancel("part never came", at(day(2026, 11, 18), 9))
        .unwrap();
    assert!(matches!(
        working.status(),
        RepairOrderStatus::Cancelled { .. }
    ));
    assert_eq!(working.started_at(), Some(when)); // the work did start
    // cancelled is final
    assert_eq!(
        working.start(when).unwrap_err().code(),
        "REPAIR_ORDER_NOT_OPEN"
    );
    assert_eq!(
        working.complete(when).unwrap_err().code(),
        "REPAIR_ORDER_NOT_IN_PROGRESS"
    );
    assert_eq!(
        working.cancel("again", when).unwrap_err().code(),
        "REPAIR_ORDER_CLOSED"
    );
}

#[test]
fn a_cancellation_needs_a_reason_that_is_not_blank_and_not_too_long() {
    let when = at(day(2026, 11, 17), 9);
    for blank in ["", "   ", "\n\t"] {
        let mut order = open_order();
        assert_eq!(
            order.cancel(blank, when).unwrap_err().code(),
            "MISSING_FIELD"
        );
        assert_eq!(order.status(), &RepairOrderStatus::Open);
        assert_eq!(order.closed_at(), None);
    }
    let mut order = open_order();
    let long = "x".repeat(501);
    assert_eq!(
        order.cancel(&long, when).unwrap_err().code(),
        "CANCELLATION_REASON_TOO_LONG"
    );
    assert_eq!(order.status(), &RepairOrderStatus::Open);
    // exactly 500 characters is fine (counted in characters, not bytes)
    order.cancel(&"é".repeat(500), when).unwrap();
}

#[test]
fn the_cancellation_reason_is_not_repeated_in_an_error_and_the_label_never_holds_it() {
    let status = RepairOrderStatus::Cancelled {
        reason: String::from("ZZmarker9137"),
    };
    assert_eq!(status.label(), "cancelled");
    let mut order = open_order();
    let error = order
        .cancel(&"ZZmarker9137".repeat(100), opened_on())
        .unwrap_err();
    assert!(!error.to_string().contains("ZZmarker9137"));
}

#[test]
fn the_due_date_can_move_or_be_cleared_but_not_before_the_day_it_was_opened() {
    let mut order = open_order(); // opened 2026-11-17
    order.set_due_date(Some(day(2026, 11, 17))).unwrap(); // the same day is fine
    assert_eq!(order.due_date(), Some(day(2026, 11, 17)));
    order.set_due_date(Some(day(2027, 1, 5))).unwrap();
    order.set_due_date(None).unwrap();
    assert_eq!(order.due_date(), None);
    let before = day(2026, 11, 17) - Days::new(1);
    assert_eq!(
        order.set_due_date(Some(before)).unwrap_err().code(),
        "DUE_DATE_BEFORE_OPENED"
    );
    assert_eq!(order.due_date(), None); // a refused change changes nothing
    // work in progress can still be moved
    order.start(opened_on()).unwrap();
    order.set_due_date(Some(day(2026, 12, 24))).unwrap();
}

#[test]
fn statuses_have_one_word_labels_and_know_when_they_are_closed() {
    let cancelled = RepairOrderStatus::Cancelled {
        reason: String::from("no"),
    };
    let all = [
        (RepairOrderStatus::Open, "open", false),
        (RepairOrderStatus::InProgress, "in_progress", false),
        (RepairOrderStatus::Completed, "completed", true),
        (cancelled, "cancelled", true),
    ];
    for (status, label, closed) in all {
        assert_eq!((status.label(), status.is_closed()), (label, closed));
    }
}

#[test]
fn the_error_kinds_tell_callers_who_can_fix_what() {
    let business = [
        "QUOTE_NOT_ACCEPTED",
        "REPAIR_ORDER_EXISTS",
        "REPAIR_ORDER_NOT_OPEN",
        "REPAIR_ORDER_NOT_IN_PROGRESS",
        "REPAIR_ORDER_CLOSED",
        "REPAIR_ORDER_CHANGED_MEANWHILE",
        "REPAIR_ORDER_NOT_SAVED",
        "REPAIR_ORDER_NUMBERS_EXHAUSTED",
    ];
    let errors = [
        QuoteError::QuoteNotAccepted,
        QuoteError::RepairOrderExists,
        QuoteError::RepairOrderNotOpen,
        QuoteError::RepairOrderNotInProgress,
        QuoteError::RepairOrderClosed,
        QuoteError::RepairOrderChangedMeanwhile,
        QuoteError::RepairOrderNotSaved,
        QuoteError::RepairOrderNumbersExhausted { year: 2026 },
    ];
    for (error, code) in errors.iter().zip(business) {
        assert_eq!(
            (error.code(), error.kind()),
            (code, ErrorKind::BusinessRule)
        );
    }
    for error in [
        QuoteError::MeterReadingRequired,
        QuoteError::DueDateBeforeOpened,
        QuoteError::CancellationReasonTooLong { max: 500 },
    ] {
        assert_eq!(error.kind(), ErrorKind::InvalidInput, "{}", error.code());
    }
    assert_eq!(
        QuoteError::StoredRepairOrderDamaged.kind(),
        ErrorKind::ServiceUnavailable
    );
    assert_eq!(
        QuoteError::UnknownRepairOrder.code(),
        "UNKNOWN_REPAIR_ORDER"
    );
}
