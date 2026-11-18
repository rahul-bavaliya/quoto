// src/quote/totals.rs: what a quote adds up to (the math; `print.rs` shows it)

use super::{LineKind, Quote};
use crate::province::calculate_tax;

impl Quote {
    /// Adds up every line of one kind (e.g. all the parts).
    pub fn total_for(&self, kind: &LineKind) -> i64 {
        self.items
            .iter()
            .filter(|item| item.kind() == kind)
            .map(|item| item.line_total())
            .sum()
    }

    pub fn labour_total(&self) -> i64 {
        self.total_for(&LineKind::Labour)
    }

    pub fn parts_total(&self) -> i64 {
        self.total_for(&LineKind::Part)
    }

    pub fn fees_total(&self) -> i64 {
        self.total_for(&LineKind::Fee)
    }

    /// Credits are negative, so this is zero or below.
    pub fn credit_total(&self) -> i64 {
        self.total_for(&LineKind::Credit)
    }

    pub fn subtotal(&self) -> i64 {
        self.items.iter().map(|item| item.line_total()).sum()
    }

    pub fn gst_amount(&self) -> i64 {
        calculate_tax(self.subtotal(), self.province().gst_rate())
    }

    pub fn pst_amount(&self) -> i64 {
        match self.province().pst_rate() {
            Some(pst_rate) => calculate_tax(self.subtotal(), pst_rate),
            None => 0,
        }
    }

    pub fn total(&self) -> i64 {
        self.subtotal() + self.gst_amount() + self.pst_amount()
    }
}
