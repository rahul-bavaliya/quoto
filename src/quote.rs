// src/quote.rs

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

    pub fn add_item(&mut self, item: LineItem) {
        self.items.push(item);
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
                "[{}] {}: {} x ${:.2} = ${:.2}",
                item.kind().label(),
                item.description(),
                item.quantity(),
                item.unit_price_cents() as f64 / 100.0,
                item.line_total() as f64 / 100.0
            );
        }
        println!("{}", "-".repeat(20));
        println!("Labour Total: {}", format_money(self.labour_total()));
        println!("Parts Total: {}", format_money(self.parts_total()));
        println!("Fees Total: {}", format_money(self.fees_total()));
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
    fn sample_quote(province: Province) -> Quote {
        let mut quote: Quote = Quote::new(province);
        // 5 add_item calls
        quote.add_item(
            LineItem::new(String::from("Oil Filter"), 1.0, 2500, LineKind::Part).unwrap(),
        );
        quote.add_item(
            LineItem::new(
                String::from("Oil Change Labour"),
                0.5,
                16000,
                LineKind::Labour,
            )
            .unwrap(),
        );
        quote
            .add_item(LineItem::new(String::from("Engine Oil"), 6.0, 850, LineKind::Part).unwrap());
        quote.add_item(
            LineItem::new(String::from("Shop Supplies"), 1.0, 1000, LineKind::Fee).unwrap(),
        );
        quote.add_item(
            LineItem::new(String::from("Air Filter"), 1.0, 3200, LineKind::Part).unwrap(),
        );
        quote.add_item(LineItem::new(String::from("DISCOUNT"), 1.0, 0, LineKind::Credit).unwrap());
        quote
    }

    #[test]
    fn sk_quote_subtotal() {
        let quote: Quote = sample_quote(Province::Saskatchewan);
        assert_eq!(quote.subtotal(), 19800);
    }

    #[test]
    fn ab_quote_has_no_pst() {
        let quote: Quote = sample_quote(Province::Alberta);
        assert_eq!(quote.pst_amount(), 0);
    }

    #[test]
    fn sk_quote_gst_cents() {
        let quote: Quote = sample_quote(Province::Saskatchewan);
        assert_eq!(quote.gst_amount(), 990);
    }

    #[test]
    fn sk_quote_pst() {
        let quote: Quote = sample_quote(Province::Saskatchewan);
        assert_eq!(quote.pst_amount(), 1188);
    }

    #[test]
    fn sk_quote_total() {
        let quote: Quote = sample_quote(Province::Saskatchewan);
        assert_eq!(quote.total(), 21978);
    }

    #[test]
    fn ab_quote_total() {
        let quote: Quote = sample_quote(Province::Alberta);
        assert_eq!(quote.total(), 20790); // subtotal + gst
    }

    #[test]
    fn new_quote_is_empty() {
        let quote: Quote = Quote::new(Province::Alberta);
        assert_eq!(quote.items.len(), 0);
    }

    #[test]
    fn sk_quote_credit_total() {
        let quote: Quote = sample_quote(Province::Saskatchewan);
        assert_eq!(quote.total(), 21978);
    }
}
