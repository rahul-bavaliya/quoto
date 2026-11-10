// src/quote.rs

use crate::line_item::{LineItem, LineKind};
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
        println!("\n{} Quote: {}", "=".repeat(5), self.province.label());

        for item in &self.items {
            println!(
                "[{}] {}: {} x ${:.2} = ${:.2}",
                item.kind().label(),
                item.description(),
                item.quantity(),
                item.unit_price(),
                item.line_total()
            );
        }
        println!("{}", "-".repeat(20));
        println!("Labour Total: ${:.2}", self.labour_total());
        println!("Parts Total: ${:.2}", self.parts_total());
        println!("Fees Total: ${:.2}", self.fees_total());
        println!("{}", "-".repeat(20));
        println!("Subtotal: ${:.2}", self.subtotal());
        println!("{}", "-".repeat(20));
        println!(
            "GST {}%: ${:.2}",
            self.province.gst_rate(),
            self.gst_amount()
        );
        if let Some(pst_rate) = self.province.pst_rate() {
            println!("PST {}%: ${:.2}", pst_rate, self.pst_amount());
        }
        println!("Total: ${:.2}", self.total());
    }

    pub fn labour_total(&self) -> f64 {
        self.items
            .iter()
            .filter(|item| matches!(item.kind(), LineKind::Labour))
            .map(|item| item.line_total())
            .sum()
    }

    pub fn parts_total(&self) -> f64 {
        self.items
            .iter()
            .filter(|item| matches!(item.kind(), LineKind::Part))
            .map(|item| item.line_total())
            .sum()
    }

    pub fn fees_total(&self) -> f64 {
        self.items
            .iter()
            .filter(|item| matches!(item.kind(), LineKind::Fee))
            .map(|item| item.line_total())
            .sum()
    }

    pub fn subtotal(&self) -> f64 {
        self.labour_total() + self.parts_total() + self.fees_total()
    }

    pub fn gst_amount(&self) -> f64 {
        calculate_tax(self.subtotal(), self.province.gst_rate())
    }

    pub fn pst_amount(&self) -> f64 {
        if let Some(pst_rate) = self.province.pst_rate() {
            calculate_tax(self.subtotal(), pst_rate)
        } else {
            0.0
        }
    }

    pub fn total(&self) -> f64 {
        calculate_total(self.subtotal(), self.gst_amount() + self.pst_amount())
    }
}

#[cfg(test)] // only build this room when running `cargo test`
mod quote_tests {
    use super::*; // bring in everything from the file above

    #[test]
    fn saskatchewan_quote_has_correct_amounts() {
        let mut sk_quote = Quote::new(Province::Saskatchewan);
        sk_quote.add_item(LineItem::new(
            String::from("Oil Filter"),
            1.0,
            25.0,
            LineKind::Part,
        ));
        sk_quote.add_item(LineItem::new(
            String::from("Oil Change Labour"),
            0.5,
            160.0,
            LineKind::Labour,
        ));
        sk_quote.add_item(LineItem::new(
            String::from("Engine Oil"),
            6.0,
            8.50,
            LineKind::Part,
        ));
        sk_quote.add_item(LineItem::new(
            String::from("Shop Supplies"),
            1.0,
            10.0,
            LineKind::Fee,
        ));
        sk_quote.add_item(LineItem::new(
            String::from("Air Filter"),
            1.0,
            32.0,
            LineKind::Part,
        ));
        assert_eq!(sk_quote.subtotal(), 198.0);
        assert_eq!(sk_quote.gst_amount(), 9.90);
        assert_eq!(sk_quote.pst_amount(), 11.88);
        assert_eq!(sk_quote.total(), 219.78);
        assert_eq!(sk_quote.province.gst_rate(), 5.0);
        assert_eq!(sk_quote.province.pst_rate(), Some(6.0));
    }
}
