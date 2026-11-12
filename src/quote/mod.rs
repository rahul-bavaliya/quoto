// src/quote/mod.rs

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

#[cfg(test)]
mod tests;
