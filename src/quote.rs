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
