// tests/quote.rs: tests for the quote feature (public API only)

use quoto::*;

// ===== quote =====

/// A customer who lives in the given province.
fn sample_customer(province: Province) -> Customer {
    samples::customer_in(province).unwrap()
}

fn sample_equipment() -> Equipment {
    samples::corolla().unwrap()
}

/// A new quote with the sample number and date, made at the branch in `province`,
/// with the sample customer and machine already added but no lines yet.
fn empty_quote(province: Province) -> Quote {
    let mut quote = quick_quote(province);
    quote.attach_customer(sample_customer(province)).unwrap();
    quote.attach_equipment(sample_equipment()).unwrap();
    quote
}

/// A quick quote: just the branch, no customer and no machine.
fn quick_quote(province: Province) -> Quote {
    Quote::new(
        samples::quote_number(),
        samples::quote_time(),
        samples::branch_in(province).unwrap(),
    )
    .unwrap()
}

// The 5 standard lines. No credits here: credit robots add their own.
fn sample_quote(province: Province) -> Quote {
    samples::oil_change_quote(province).unwrap()
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
    let quote = empty_quote(Province::Alberta);
    assert_eq!(quote.subtotal(), 0);
    assert_eq!(quote.total(), 0);
}

#[test]
fn a_quote_knows_its_number_and_its_date() {
    let quote = empty_quote(Province::Saskatchewan);
    assert_eq!(quote.number().to_string(), "Q-2026-00000001");
    assert!(quote.to_string().contains("Date: 2026-11-16 09:30:00"));
}

#[test]
fn the_printed_header_shows_the_number_and_the_date() {
    let text = empty_quote(Province::Saskatchewan).to_string();
    assert!(text.contains("===== Quote Q-2026-00000001: Saskatchewan ====="));
    assert!(text.contains("Date: 2026-11-16 09:30:00"));
}

#[test]
fn quote_takes_its_province_from_the_branch_not_the_customer() {
    // An Alberta branch quoting a Saskatchewan customer: the branch decides the taxes.
    let mut quote = quick_quote(Province::Alberta);
    quote
        .attach_customer(sample_customer(Province::Saskatchewan))
        .unwrap();
    assert_eq!(quote.province(), &Province::Alberta);
    assert_eq!(quote.branch().province(), &Province::Alberta);
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

// ===== line item =====

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

// ===== quote number =====

#[test]
fn new_builds_a_valid_number() {
    let number = QuoteNumber::new(2026, 1).unwrap();
    assert_eq!(number.to_string(), "Q-2026-00000001");
}

#[test]
fn it_prints_with_leading_zeros() {
    let number = QuoteNumber::new(2026, 42).unwrap();
    assert_eq!(number.to_string(), "Q-2026-00000042");
}

#[test]
fn counter_zero_is_refused() {
    assert!(QuoteNumber::new(2026, 0).is_err());
}

#[test]
fn a_counter_over_99_999_999_is_refused() {
    assert!(QuoteNumber::new(2026, 100_000_000).is_err());
}

#[test]
fn the_highest_counter_is_accepted() {
    assert!(QuoteNumber::new(2026, 99_999_999).is_ok());
}

#[test]
fn a_year_that_is_not_four_digits_is_refused() {
    assert!(QuoteNumber::new(26, 1).is_err());
    assert!(QuoteNumber::new(10000, 1).is_err());
}

#[test]
fn the_error_remembers_the_number_it_refused() {
    assert_eq!(
        QuoteNumber::new(26, 1),
        Err(QuoteError::InvalidQuoteNumber {
            input: String::from("Q-26-00000001")
        })
    );
}

#[test]
fn the_first_quote_of_a_year_is_counter_one() {
    let number = QuoteNumber::first_of_year(2027).unwrap();
    assert_eq!(number.to_string(), "Q-2027-00000001");
}

#[test]
fn next_adds_one_in_the_same_year() {
    let number = QuoteNumber::new(2026, 41).unwrap();
    assert_eq!(number.next().unwrap().to_string(), "Q-2026-00000042");
}

#[test]
fn next_after_the_last_number_is_refused_not_wrapped() {
    let last = QuoteNumber::new(2026, 99_999_999).unwrap();
    assert_eq!(
        last.next(),
        Err(QuoteError::QuoteNumbersExhausted { year: 2026 })
    );
}

// ===== quote numbers (the ticket machine) =====

#[test]
fn the_first_press_gives_number_one_of_the_year() {
    let mut machine = InMemoryQuoteNumbers::for_year(2026);
    assert_eq!(
        machine.next_number().unwrap().to_string(),
        "Q-2026-00000001"
    );
}

#[test]
fn each_press_gives_the_next_number() {
    let mut machine = InMemoryQuoteNumbers::for_year(2026);
    let first = machine.next_number().unwrap();
    let second = machine.next_number().unwrap();
    let third = machine.next_number().unwrap();
    assert_eq!(first.to_string(), "Q-2026-00000001");
    assert_eq!(second.to_string(), "Q-2026-00000002");
    assert_eq!(third.to_string(), "Q-2026-00000003");
}

#[test]
fn two_machines_do_not_share_a_count() {
    let mut one = InMemoryQuoteNumbers::for_year(2026);
    let mut other = InMemoryQuoteNumbers::for_year(2027);
    one.next_number().unwrap();
    one.next_number().unwrap();
    assert_eq!(other.next_number().unwrap().to_string(), "Q-2027-00000001");
}

#[test]
fn a_machine_with_a_bad_year_refuses_to_start() {
    let mut machine = InMemoryQuoteNumbers::for_year(26);
    assert!(matches!(
        machine.next_number(),
        Err(QuoteError::InvalidQuoteNumber { .. })
    ));
}

#[test]
fn the_sample_machine_starts_at_the_sample_number() {
    let mut machine = samples::quote_numbers();
    assert_eq!(machine.next_number().unwrap(), samples::quote_number());
}

#[test]
fn a_new_machine_uses_the_current_year() {
    let number = InMemoryQuoteNumbers::new()
        .next_number()
        .unwrap()
        .to_string();
    // "Q-2026-00000001": the 4 characters after "Q-" are the year.
    let year: u16 = number[2..6].parse().unwrap();
    assert!(year >= 2026);
    assert!(number.ends_with("-00000001"));
}

// ===== expiry date and promise date =====

use chrono::{Days, NaiveDate};

/// The quote's own date, from the sample time.
fn quote_day() -> NaiveDate {
    samples::quote_time().date()
}

fn day(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

#[test]
fn a_new_quote_is_good_for_thirty_days() {
    let quote = empty_quote(Province::Saskatchewan);
    assert_eq!(quote.expires_on(), quote_day() + Days::new(30));
}

#[test]
fn the_thirty_days_count_across_the_end_of_the_year() {
    // 2026-12-20 + 30 days = 2027-01-19 (11 days left in December, then 19 in January)
    let quote = Quote::new(
        samples::quote_number(),
        day(2026, 12, 20).and_hms_opt(16, 45, 0).unwrap(),
        samples::branch_in(Province::Saskatchewan).unwrap(),
    )
    .unwrap();
    assert_eq!(quote.expires_on(), day(2027, 1, 19));
}

#[test]
fn the_time_of_day_does_not_change_the_expiry_date() {
    let late = Quote::new(
        samples::quote_number(),
        day(2026, 11, 16).and_hms_opt(23, 59, 59).unwrap(),
        samples::branch_in(Province::Saskatchewan).unwrap(),
    )
    .unwrap();
    assert_eq!(
        late.expires_on(),
        empty_quote(Province::Saskatchewan).expires_on()
    );
}

#[test]
fn the_expiry_date_can_be_changed_to_a_later_or_earlier_day() {
    let mut quote = empty_quote(Province::Saskatchewan);
    quote.set_expiry(quote_day() + Days::new(60)).unwrap();
    assert_eq!(quote.expires_on(), quote_day() + Days::new(60));
    quote.set_expiry(quote_day() + Days::new(7)).unwrap();
    assert_eq!(quote.expires_on(), quote_day() + Days::new(7));
}

#[test]
fn a_quote_can_be_good_for_its_own_day_only() {
    let mut quote = empty_quote(Province::Saskatchewan);
    quote.set_expiry(quote_day()).unwrap();
    assert_eq!(quote.expires_on(), quote_day());
}

#[test]
fn an_expiry_date_before_the_quote_date_is_refused_and_nothing_changes() {
    let mut quote = empty_quote(Province::Saskatchewan);
    let before = quote_day() - Days::new(1);
    assert_eq!(
        quote.set_expiry(before),
        Err(QuoteError::ExpiryBeforeQuoteDate {
            date: before,
            quote_date: quote_day()
        })
    );
    assert_eq!(quote.expires_on(), quote_day() + Days::new(30));
}

#[test]
fn a_new_quote_has_no_promise_date() {
    assert_eq!(empty_quote(Province::Saskatchewan).promise_date(), None);
}

#[test]
fn a_promise_date_can_be_set_and_changed() {
    let mut quote = empty_quote(Province::Saskatchewan);
    quote.set_promise_date(quote_day() + Days::new(3)).unwrap();
    assert_eq!(quote.promise_date(), Some(quote_day() + Days::new(3)));
    quote.set_promise_date(quote_day() + Days::new(5)).unwrap();
    assert_eq!(quote.promise_date(), Some(quote_day() + Days::new(5)));
}

#[test]
fn a_machine_can_be_promised_for_the_quote_day_itself() {
    let mut quote = empty_quote(Province::Saskatchewan);
    quote.set_promise_date(quote_day()).unwrap();
    assert_eq!(quote.promise_date(), Some(quote_day()));
}

#[test]
fn a_promise_date_before_the_quote_date_is_refused_and_nothing_changes() {
    let mut quote = empty_quote(Province::Saskatchewan);
    let before = quote_day() - Days::new(1);
    assert_eq!(
        quote.set_promise_date(before),
        Err(QuoteError::PromiseBeforeQuoteDate {
            date: before,
            quote_date: quote_day()
        })
    );
    assert_eq!(quote.promise_date(), None);
}

#[test]
fn a_promise_date_may_be_after_the_prices_expire() {
    // The expiry is about the prices, the promise is about the work.
    let mut quote = empty_quote(Province::Saskatchewan);
    let late = quote.expires_on() + Days::new(10);
    quote.set_promise_date(late).unwrap();
    assert_eq!(quote.promise_date(), Some(late));
}

#[test]
fn the_printed_quote_shows_the_expiry_and_the_promise_date() {
    let mut quote = empty_quote(Province::Saskatchewan);
    quote.set_expiry(day(2026, 12, 16)).unwrap();
    quote.set_promise_date(day(2026, 11, 20)).unwrap();
    let text = quote.to_string();
    assert!(text.contains("Valid until: 2026-12-16\n"));
    assert!(text.contains("Promise date: 2026-11-20\n"));
}

#[test]
fn a_quote_without_a_promise_date_says_it_is_to_be_confirmed() {
    let text = empty_quote(Province::Saskatchewan).to_string();
    assert!(text.contains("Valid until: 2026-12-16\n"));
    assert!(text.contains("Promise date: to be confirmed\n"));
}

#[test]
fn the_dates_come_right_after_the_quote_date_in_the_header() {
    let text = empty_quote(Province::Saskatchewan).to_string();
    let date = text.find("Date: 2026-11-16 09:30:00").unwrap();
    let valid = text.find("Valid until:").unwrap();
    let promise = text.find("Promise date:").unwrap();
    let customer = text.find("Customer:").unwrap();
    assert!(date < valid && valid < promise && promise < customer);
}

// ===== id (given by the database, never made by Quoto) =====

fn some_id() -> uuid::Uuid {
    uuid::Uuid::from_u128(0x5555)
}

#[test]
fn a_new_quote_has_no_id() {
    assert_eq!(empty_quote(Province::Saskatchewan).id(), None);
}

#[test]
fn a_quote_loaded_from_the_database_has_its_id() {
    let quote = empty_quote(Province::Saskatchewan).with_id(some_id());
    assert_eq!(quote.id(), Some(some_id()));
}

#[test]
fn the_quote_id_stays_the_same_while_the_quote_changes() {
    let mut quote = empty_quote(Province::Saskatchewan).with_id(some_id());
    quote.add_item(credit(1000)).unwrap_err(); // refused: a credit on an empty quote
    quote
        .add_item(LineItem::new(String::from("Part"), 1.0, 500, LineKind::Part).unwrap())
        .unwrap();
    quote
        .set_promise_date(samples::quote_time().date())
        .unwrap();
    quote.reject("Too expensive").unwrap();
    quote.reopen().unwrap();
    quote.accept(samples::quote_time().date()).unwrap();
    assert_eq!(quote.id(), Some(some_id()));
}

#[test]
fn the_quote_id_is_not_the_customers_or_the_equipments_id() {
    let quote = empty_quote(Province::Saskatchewan).with_id(some_id());
    assert_eq!(quote.customer().unwrap().id(), None);
    assert_eq!(quote.equipment().unwrap().id(), None);
}

#[test]
fn the_printed_quote_never_shows_the_id_only_the_quote_number() {
    let with = empty_quote(Province::Saskatchewan).with_id(some_id());
    assert_eq!(
        with.to_string(),
        empty_quote(Province::Saskatchewan).to_string()
    );
    assert!(!with.to_string().contains(&some_id().to_string()));
}

// ===== line ids (the line's own id, and the quote it belongs to) =====

fn part_line() -> LineItem {
    LineItem::new(String::from("Oil filter"), 1.0, 500, LineKind::Part).unwrap()
}

fn line_id() -> uuid::Uuid {
    uuid::Uuid::from_u128(0x7777)
}

#[test]
fn a_new_line_has_no_id_and_no_quote_id() {
    let line = part_line();
    assert_eq!(line.id(), None);
    assert_eq!(line.quote_id(), None);
}

#[test]
fn a_line_loaded_from_the_database_has_its_id() {
    let line = part_line().with_id(line_id());
    assert_eq!(line.id(), Some(line_id()));
    assert_eq!(line.quote_id(), None); // only a quote sets this
}

#[test]
fn a_line_added_to_a_quote_without_an_id_has_no_quote_id() {
    let mut quote = empty_quote(Province::Saskatchewan);
    quote.add_item(part_line()).unwrap();
    assert_eq!(quote.items()[0].quote_id(), None);
}

#[test]
fn a_line_added_to_a_quote_with_an_id_gets_that_quote_id() {
    let mut quote = empty_quote(Province::Saskatchewan).with_id(some_id());
    quote.add_item(part_line()).unwrap();
    assert_eq!(quote.items()[0].quote_id(), Some(some_id()));
    assert_eq!(quote.items()[0].id(), None); // the line's own id is still not given
}

#[test]
fn lines_already_on_a_quote_get_the_quote_id_when_it_is_given() {
    let mut quote = empty_quote(Province::Saskatchewan);
    quote.add_item(part_line()).unwrap();
    quote.add_item(part_line()).unwrap();
    let quote = quote.with_id(some_id());
    assert!(
        quote
            .items()
            .iter()
            .all(|line| line.quote_id() == Some(some_id()))
    );
}

#[test]
fn a_line_keeps_its_own_id_when_it_joins_a_quote() {
    let mut quote = empty_quote(Province::Saskatchewan).with_id(some_id());
    quote.add_item(part_line().with_id(line_id())).unwrap();
    assert_eq!(quote.items()[0].id(), Some(line_id()));
}

#[test]
fn a_refused_line_is_not_added_and_nothing_changes() {
    let mut quote = empty_quote(Province::Saskatchewan).with_id(some_id());
    quote.add_item(credit(1000)).unwrap_err();
    assert!(quote.items().is_empty());
}

#[test]
fn lines_come_back_in_the_order_they_were_added() {
    let mut quote = empty_quote(Province::Saskatchewan);
    for name in ["First", "Second", "Third"] {
        let line = LineItem::new(String::from(name), 1.0, 100, LineKind::Part).unwrap();
        quote.add_item(line).unwrap();
    }
    let names: Vec<&str> = quote.items().iter().map(|l| l.description()).collect();
    assert_eq!(names, ["First", "Second", "Third"]);
}

#[test]
fn line_ids_never_change_the_printed_quote_or_the_totals() {
    let mut plain = empty_quote(Province::Saskatchewan);
    plain.add_item(part_line()).unwrap();
    let mut with_ids = empty_quote(Province::Saskatchewan).with_id(some_id());
    with_ids.add_item(part_line().with_id(line_id())).unwrap();
    assert_eq!(with_ids.to_string(), plain.to_string());
    assert_eq!(with_ids.total(), plain.total());
    assert!(!with_ids.to_string().contains(&line_id().to_string()));
}

// ===== reading a number back =====

#[test]
fn a_quote_number_is_read_back_from_its_printed_form() {
    for (year, counter) in [(2026, 1), (2026, 42), (1000, 99_999_999), (9999, 1)] {
        let number = QuoteNumber::new(year, counter).unwrap();
        let back = QuoteNumber::parse(&number.to_string()).unwrap();
        assert_eq!(back, number);
        assert_eq!((back.year(), back.counter()), (year, counter));
    }
}

#[test]
fn text_that_is_not_a_quote_number_is_refused() {
    for text in [
        "",
        "Q-2026-1",
        "Q-2026-000000001",
        "Q-26-00000001",
        "q-2026-00000001",
        "Q-2026-0000000a",
        "Q-+026-00000001",
        "Q-2026-00000000",
        "Q-0999-00000001",
        "Q-2026-00000001-",
        " Q-2026-00000001",
        "X-2026-00000001",
        "Q2026-00000001",
    ] {
        assert!(
            matches!(
                QuoteNumber::parse(text),
                Err(QuoteError::InvalidQuoteNumber { .. })
            ),
            "{text:?}"
        );
    }
}

// ===== line limits (Day 53) =====

fn line(description: &str, quantity: f64, price: i64) -> QuoteResult<LineItem> {
    LineItem::new(description.to_string(), quantity, price, LineKind::Part)
}

#[test]
fn a_line_with_no_number_for_a_quantity_is_refused() {
    for quantity in [f64::NAN, f64::NEG_INFINITY, 0.0, -1.0] {
        assert!(matches!(
            line("Filter", quantity, 100),
            Err(QuoteError::BadQuantity { .. })
        ));
    }
    // infinity is not "a number of items" either: it is too big
    assert!(matches!(
        line("Filter", f64::INFINITY, 100),
        Err(QuoteError::LineTooLarge { .. })
    ));
}

#[test]
fn a_line_too_big_to_add_up_safely_is_refused_and_the_limits_themselves_are_allowed() {
    assert!(matches!(
        line("Filter", MAX_QUANTITY + 1.0, 100),
        Err(QuoteError::LineTooLarge { .. })
    ));
    assert!(matches!(
        line("Filter", 1.0, MAX_UNIT_PRICE_CENTS + 1),
        Err(QuoteError::LineTooLarge { .. })
    ));
    let biggest = line("Filter", MAX_QUANTITY, MAX_UNIT_PRICE_CENTS).unwrap();
    assert_eq!(biggest.line_total(), 100_000_000_000_000);
    // ...and a quote of many of the biggest still adds up without overflowing
    let mut quote = quick_quote(Province::Saskatchewan);
    for _ in 0..MAX_LINES {
        quote
            .add_item(line("Filter", MAX_QUANTITY, MAX_UNIT_PRICE_CENTS).unwrap())
            .unwrap();
    }
    assert_eq!(quote.subtotal(), 200 * 100_000_000_000_000);
}

#[test]
fn a_description_is_trimmed_not_blank_and_not_too_long() {
    assert_eq!(
        line("  Oil filter  ", 1.0, 100).unwrap().description(),
        "Oil filter"
    );
    for blank in ["", "   ", "\t\n"] {
        assert!(matches!(
            line(blank, 1.0, 100),
            Err(QuoteError::MissingField {
                field: "description"
            })
        ));
    }
    assert!(line(&"x".repeat(MAX_DESCRIPTION_CHARS), 1.0, 100).is_ok());
    assert!(matches!(
        line(&"x".repeat(MAX_DESCRIPTION_CHARS + 1), 1.0, 100),
        Err(QuoteError::DescriptionTooLong {
            max: MAX_DESCRIPTION_CHARS
        })
    ));
    // counted in characters, not bytes
    assert!(line(&"é".repeat(MAX_DESCRIPTION_CHARS), 1.0, 100).is_ok());
}

#[test]
fn a_quote_holds_at_most_its_limit_of_lines() {
    let mut quote = quick_quote(Province::Saskatchewan);
    for _ in 0..MAX_LINES {
        quote.add_item(line("Filter", 1.0, 100).unwrap()).unwrap();
    }
    assert!(matches!(
        quote.add_item(line("One more", 1.0, 100).unwrap()),
        Err(QuoteError::TooManyLines { max: MAX_LINES })
    ));
    assert_eq!(quote.items().len(), MAX_LINES);
}
