// tests/quote_pdf.rs: a quote as a PDF file (no database)

use chrono::NaiveDate;
use quoto::*;

// ---- reading a PDF back (just enough for these files: plain text streams) ----

fn find(haystack: &[u8], needle: &str, from: usize) -> Option<usize> {
    haystack[from..]
        .windows(needle.len())
        .position(|w| w == needle.as_bytes())
        .map(|at| at + from)
}

/// Every object's text, in order, checked against the cross-reference table on the way.
fn objects(pdf: &[u8]) -> Vec<String> {
    assert!(pdf.starts_with(b"%PDF-1.4\n"), "header");
    assert!(pdf.ends_with(b"%%EOF\n"), "end marker");
    let start = find(pdf, "startxref\n", 0).expect("startxref") + "startxref\n".len();
    let end = find(pdf, "\n", start).unwrap();
    let xref: usize = std::str::from_utf8(&pdf[start..end])
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(
        &pdf[xref..xref + 5],
        b"xref\n",
        "startxref points at the table"
    );
    let table = String::from_utf8_lossy(&pdf[xref..]).to_string();
    let mut lines = table.lines();
    lines.next(); // xref
    let count: usize = lines
        .next()
        .unwrap()
        .split(' ')
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(lines.next().unwrap(), "0000000000 65535 f ");
    let mut out = Vec::new();
    for number in 1..count {
        let entry = lines.next().unwrap();
        assert_eq!(entry.len(), 19, "{entry:?} (20 bytes with the newline)");
        assert!(entry.ends_with(" 00000 n "), "{entry}");
        let offset: usize = entry[..10].parse().unwrap();
        let head = format!("{number} 0 obj\n");
        assert_eq!(
            &pdf[offset..offset + head.len()],
            head.as_bytes(),
            "object {number}"
        );
        let body_end = find(pdf, "\nendobj\n", offset).unwrap();
        out.push(String::from_utf8_lossy(&pdf[offset + head.len()..body_end]).to_string());
    }
    assert!(table.contains(&format!("/Size {count}")));
    out
}

/// The text of one page's drawing instructions, with its stream length checked.
fn streams(pdf: &[u8]) -> Vec<String> {
    objects(pdf)
        .into_iter()
        .filter(|object| object.contains("\nstream\n"))
        .map(|object| {
            let length: usize = object
                .split("/Length ")
                .nth(1)
                .unwrap()
                .split(' ')
                .next()
                .unwrap()
                .parse()
                .unwrap();
            let body = object.split("\nstream\n").nth(1).unwrap();
            let body = body.strip_suffix("endstream").expect("endstream");
            assert_eq!(body.len(), length, "/Length is the real length");
            body.to_string()
        })
        .collect()
}

/// The words shown on a page, in the order they were drawn (the `(...) Tj` strings, un-escaped).
fn shown(stream: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in stream.lines().filter(|l| l.ends_with("Tj ET")) {
        let open = line.find('(').unwrap();
        let close = line.rfind(')').unwrap();
        let inner = &line[open + 1..close];
        let mut text = String::new();
        let mut chars = inner.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '\\' {
                text.push(c);
                continue;
            }
            match chars.next().unwrap() {
                d @ '0'..='7' => {
                    let mut octal = d.to_string();
                    for _ in 0..2 {
                        octal.push(chars.next().unwrap());
                    }
                    text.push(char::from(u8::from_str_radix(&octal, 8).unwrap()));
                }
                other => text.push(other),
            }
        }
        out.push(text);
    }
    out
}

fn all_text(pdf: &[u8]) -> Vec<String> {
    streams(pdf).iter().flat_map(|s| shown(s)).collect()
}

fn day(year: i32, month: u32, date: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, date).unwrap()
}

fn sample(province: Province) -> Quote {
    samples::oil_change_quote(province).unwrap()
}

fn part(description: &str) -> LineItem {
    LineItem::new(description.to_string(), 1.0, 1250, LineKind::Part).unwrap()
}

fn bare() -> Quote {
    Quote::new(
        samples::quote_number(),
        samples::quote_time(),
        samples::branch_in(Province::Saskatchewan).unwrap(),
    )
    .unwrap()
}

// ---- tests ----

#[test]
fn the_file_is_a_well_formed_pdf() {
    let pdf = sample(Province::Saskatchewan).to_pdf(day(2026, 11, 18));
    let objects = objects(&pdf); // checks the header, the table of offsets and the end marker
    assert!(objects[0].contains("/Type /Catalog"));
    assert!(objects[1].contains("/Count 1") && objects[1].contains("/MediaBox [0 0 612 792]"));
    assert!(objects[2].contains("/BaseFont /Helvetica ") && objects[3].contains("/Helvetica-Bold"));
    assert!(objects[4].contains("/Title (Q-2026-00000001)"));
    // nothing but the header comment is outside plain ASCII, and the file stays small
    assert!(pdf[16..].is_ascii());
    assert!(pdf.len() < 6_000, "{} bytes", pdf.len());
}

#[test]
fn the_same_quote_on_the_same_day_gives_the_same_bytes() {
    let quote = sample(Province::Saskatchewan);
    assert_eq!(
        quote.to_pdf(day(2026, 11, 18)),
        quote.to_pdf(day(2026, 11, 18))
    );
}

#[test]
fn the_page_shows_what_the_text_print_shows() {
    let quote = sample(Province::Saskatchewan);
    let text = all_text(&quote.to_pdf(day(2026, 11, 18)));
    for wanted in [
        "QUOTE",
        "Q-2026-00000001",
        "PENDING",
        "Regina Branch",
        "Prepared for",
        "Jane Smith",
        "Equipment",
        "Toyota Corolla Cross (2025)",
        "S/N: SERIAL_NUMBER_TEST_1",
        "Oil Filter",
        "Oil Change Labour",
        "$25.00",
        "$160.00",
        "Subtotal",
        "$198.00",
        "GST 5%",
        "$9.90",
        "PST 6%",
        "$11.88",
        "Total",
        "$219.78",
        "Valid until: 2026-12-16",
        "Promise date: to be confirmed",
        "Page 1 of 1",
    ] {
        assert!(
            text.iter().any(|t| t == wanted),
            "{wanted} is missing from {text:?}"
        );
    }
    assert!(text.iter().any(|t| t.starts_with("Phone: ")));
}

#[test]
fn alberta_has_no_pst_line() {
    let text = all_text(&sample(Province::Alberta).to_pdf(day(2026, 11, 18)));
    assert!(text.iter().any(|t| t == "GST 5%"));
    assert!(!text.iter().any(|t| t.contains("PST")), "{text:?}");
}

#[test]
fn a_quote_with_no_customer_machine_or_lines_says_so() {
    let text = all_text(&bare().to_pdf(day(2026, 11, 18)));
    assert_eq!(text.iter().filter(|t| *t == "to be added").count(), 2);
    assert!(text.iter().any(|t| t == "No lines yet."));
    assert!(text.iter().any(|t| t == "$0.00"));
}

#[test]
fn a_promise_date_and_a_credit_are_shown() {
    let mut quote = sample(Province::Saskatchewan);
    quote.set_promise_date(day(2026, 11, 20)).unwrap();
    quote
        .add_item(LineItem::new("Goodwill".into(), 1.0, 5000, LineKind::Credit).unwrap())
        .unwrap();
    let text = all_text(&quote.to_pdf(day(2026, 11, 18)));
    assert!(text.iter().any(|t| t == "Promise date: 2026-11-20"));
    assert!(text.iter().any(|t| t == "Goodwill"));
    assert!(text.iter().any(|t| t == "-$50.00")); // the line, and the unit price
    assert!(text.iter().any(|t| t == "CREDIT"));
}

#[test]
fn the_decision_and_the_expiry_are_marked() {
    let mut quote = sample(Province::Saskatchewan);
    let made = quote.created_at().date();
    let mark = |quote: &Quote, today| -> String {
        all_text(&quote.to_pdf(today))
            .into_iter()
            .find(|t| t.contains("PENDING") || t.contains("ACCEPTED") || t.contains("REJECTED"))
            .unwrap()
    };
    assert_eq!(mark(&quote, made), "PENDING");
    // the last good day is not expired; the day after is
    assert_eq!(mark(&quote, quote.expires_on()), "PENDING");
    assert_eq!(
        mark(&quote, quote.expires_on() + chrono::Days::new(1)),
        "PENDING - EXPIRED"
    );
    let mut rejected = sample(Province::Saskatchewan);
    rejected.reject("too dear").unwrap();
    assert_eq!(mark(&rejected, made), "REJECTED");
    // an expired quote that was decided is not called expired
    assert_eq!(
        mark(&rejected, quote.expires_on() + chrono::Days::new(1)),
        "REJECTED"
    );
    quote.accept(made).unwrap();
    assert_eq!(mark(&quote, made), "ACCEPTED");
    // the customer's own words are not printed on the customer's copy
    let text = all_text(&rejected.to_pdf(made));
    assert!(!text.iter().any(|t| t.contains("too dear")));
}

#[test]
fn a_long_quote_goes_on_to_more_pages_and_each_page_has_its_own_head_and_number() {
    let mut quote = sample(Province::Saskatchewan);
    for n in 0..120 {
        quote.add_item(part(&format!("Bolt number {n}"))).unwrap();
    }
    let pdf = quote.to_pdf(day(2026, 11, 18));
    let pages = streams(&pdf);
    assert!(pages.len() >= 3, "{} pages", pages.len());
    assert!(objects(&pdf)[1].contains(&format!("/Count {}", pages.len())));
    let mut bolts = Vec::new();
    for (index, page) in pages.iter().enumerate() {
        let text = shown(page);
        assert!(
            text.iter().any(|t| t == "Description"),
            "page {} has no column heads",
            index + 1
        );
        assert!(
            text.iter()
                .any(|t| *t == format!("Page {} of {}", index + 1, pages.len()))
        );
        assert!(text.iter().any(|t| t == "Quote Q-2026-00000001"));
        bolts.extend(text.into_iter().filter(|t| t.starts_with("Bolt number ")));
    }
    // every line is drawn once, in order, and the first page also has the head of the quote
    let expected: Vec<String> = (0..120).map(|n| format!("Bolt number {n}")).collect();
    assert_eq!(bolts, expected);
    assert!(shown(&pages[0]).iter().any(|t| t == "QUOTE"));
    assert!(!shown(&pages[1]).iter().any(|t| t == "QUOTE"));
    // the totals are together on one page (the last)
    let with_totals: Vec<usize> = pages
        .iter()
        .enumerate()
        .filter(|(_, page)| shown(page).iter().any(|t| t == "Subtotal"))
        .map(|(index, _)| index)
        .collect();
    assert_eq!(with_totals, [pages.len() - 1]);
    let last = shown(&pages[pages.len() - 1]);
    for wanted in ["GST 5%", "PST 6%", "Total"] {
        assert!(last.iter().any(|t| t == wanted), "{wanted}");
    }
}

#[test]
fn a_long_description_wraps_onto_more_lines_and_loses_nothing() {
    let mut quote = bare();
    let words = "Replace the left front hydraulic hose and clamp then pressure test the whole circuit under load \
                 and record the readings on the work sheet for the customer";
    quote.add_item(part(words)).unwrap();
    quote.add_item(part(&"W".repeat(90))).unwrap();
    let text = all_text(&quote.to_pdf(day(2026, 11, 18)));
    let described: Vec<&String> = text.iter().filter(|t| words.contains(t.as_str())).collect();
    assert!(described.len() >= 2, "{text:?}");
    let joined: Vec<&str> = described.iter().map(|t| t.as_str()).collect();
    assert_eq!(joined.join(" "), words);
    let w: String = text
        .iter()
        .filter(|t| t.starts_with('W'))
        .map(|t| t.as_str())
        .collect();
    assert_eq!(w, "W".repeat(90)); // the long word was cut, not lost
}

#[test]
fn brackets_backslashes_accents_and_other_scripts_are_safe() {
    let mut quote = bare();
    quote
        .add_item(part("Belt (A\\B) - Joint d'étanchéité"))
        .unwrap();
    quote.add_item(part("日本語")).unwrap();
    let pdf = quote.to_pdf(day(2026, 11, 18));
    let text = all_text(&pdf); // read back through the real escapes
    assert!(
        text.iter()
            .any(|t| t == "Belt (A\\B) - Joint d\u{27}\u{e9}tanch\u{e9}it\u{e9}"),
        "{text:?}"
    );
    assert!(text.iter().any(|t| t == "???")); // the standard fonts can't show them
    assert!(pdf[16..].is_ascii());
    objects(&pdf); // and the file is still well formed
}

#[test]
fn money_in_the_biggest_allowed_line_still_fits_in_its_column() {
    let mut quote = bare();
    quote
        .add_item(
            LineItem::new(
                "Huge".into(),
                MAX_QUANTITY,
                MAX_UNIT_PRICE_CENTS,
                LineKind::Part,
            )
            .unwrap(),
        )
        .unwrap();
    let text = all_text(&quote.to_pdf(day(2026, 11, 18)));
    assert!(text.iter().any(|t| t == "$1000000000000.00"), "{text:?}");
}
