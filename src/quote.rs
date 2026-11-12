// src/quote.rs

use crate::errors::QuoteError;
use crate::line_item::{LineItem, LineKind};
use crate::money::format_money;
use crate::province::Province;
use crate::tax::{calculate_tax, calculate_total};

pub struct Quote {
    province: Province,
    items: Vec<LineItem>,
}

impl Quote {
    pub fn new(province: Province) -> Self {
        Quote {
            province,
            items: Vec::new(),
        }
    }

    /// Adds a line to the quote. A credit is refused if it would push
    /// the subtotal below zero (the dealer would owe the customer).
    pub fn add_item(&mut self, item: LineItem) -> Result<(), QuoteError> {
        let subtotal_after = self.subtotal() + item.line_total();
        if matches!(item.kind(), LineKind::Credit) && subtotal_after < 0 {
            return Err(QuoteError::CreditTooLarge {
                credit_cents: -item.line_total(),
                subtotal_cents: self.subtotal(),
            });
        }

        self.items.push(item);
        Ok(())
    }

    pub fn print(&self) {
        println!(
            "\n{} Quote: {} {}",
            "=".repeat(5),
            self.province.label(),
            "=".repeat(5)
        );

        for item in &self.items {
            println!(
                "[{}] {}: {} x {} = {}",
                item.kind().label(),
                item.description(),
                item.quantity(),
                format_money(item.signed_unit_price_cents()),
                format_money(item.line_total())
            );
        }
        println!("{}", "-".repeat(20));
        println!("Labour Total: {}", format_money(self.labour_total()));
        println!("Parts Total: {}", format_money(self.parts_total()));
        println!("Fees Total: {}", format_money(self.fees_total()));
        println!("Credit Total: {}", format_money(self.credit_total()));

        println!("{}", "-".repeat(20));
        println!("Subtotal: {}", format_money(self.subtotal()));
        println!("{}", "-".repeat(20));
        println!(
            "GST {}%: {}",
            self.province.gst_rate(),
            format_money(self.gst_amount())
        );
        if let Some(pst_rate) = self.province.pst_rate() {
            println!("PST {}%: {}", pst_rate, format_money(self.pst_amount()));
        }
        println!("Total: {}", format_money(self.total()));
    }

    pub fn labour_total(&self) -> i64 {
        self.items
            .iter()
            .filter(|item| matches!(item.kind(), LineKind::Labour))
            .map(|item| item.line_total())
            .sum()
    }

    pub fn parts_total(&self) -> i64 {
        self.items
            .iter()
            .filter(|item| matches!(item.kind(), LineKind::Part))
            .map(|item| item.line_total())
            .sum()
    }

    pub fn fees_total(&self) -> i64 {
        self.items
            .iter()
            .filter(|item| matches!(item.kind(), LineKind::Fee))
            .map(|item| item.line_total())
            .sum()
    }

    pub fn credit_total(&self) -> i64 {
        self.items
            .iter()
            .filter(|item| matches!(item.kind(), LineKind::Credit))
            .map(|item| item.line_total())
            .sum()
    }

    pub fn subtotal(&self) -> i64 {
        self.labour_total() + self.parts_total() + self.fees_total() + self.credit_total()
    }

    pub fn gst_amount(&self) -> i64 {
        calculate_tax(self.subtotal(), self.province.gst_rate())
    }

    pub fn pst_amount(&self) -> i64 {
        if let Some(pst_rate) = self.province.pst_rate() {
            calculate_tax(self.subtotal(), pst_rate)
        } else {
            0
        }
    }

    pub fn total(&self) -> i64 {
        calculate_total(self.subtotal(), self.gst_amount() + self.pst_amount())
    }
}

#[cfg(test)] // only build this room when running `cargo test`
mod quote_tests {
    use super::*; // bring in everything from the file above

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
            .add_item(
                LineItem::new(String::from("Shop Supplies"), 1.0, 1000, LineKind::Fee).unwrap(),
            )
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
    fn sk_quote_total() {
        let quote = sample_quote(Province::Saskatchewan);
        assert_eq!(quote.total(), 21978);
    }

    #[test]
    fn ab_quote_total() {
        let quote = sample_quote(Province::Alberta);
        assert_eq!(quote.total(), 20790); // subtotal + gst
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
    fn sk_quote_with_50_credit() {
        let mut quote = sample_quote(Province::Saskatchewan);
        quote.add_item(credit(5000)).unwrap();
        assert_eq!(quote.total(), 16428); // 148.00 + 7.40 + 8.88
    }

    #[test]
    fn ab_quote_with_50_credit() {
        let mut quote = sample_quote(Province::Alberta);
        quote.add_item(credit(5000)).unwrap();
        assert_eq!(quote.total(), 15540); // 148.00 + 7.40
    }

    #[test]
    fn credit_bigger_than_subtotal_is_refused() {
        let mut quote = sample_quote(Province::Saskatchewan);
        let result = quote.add_item(credit(30000)); // $300 credit on a $198 job
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
    fn credit_equal_to_subtotal_is_allowed() {
        let mut quote = sample_quote(Province::Alberta);
        assert!(quote.add_item(credit(19800)).is_ok()); // a free job is fine
        assert_eq!(quote.total(), 0);
    }
}
