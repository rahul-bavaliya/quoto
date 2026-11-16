// src/quote.rs

use crate::customer::Customer;
use crate::equipment::Equipment;
use crate::errors::QuoteError;
use crate::money::format_money;
use crate::province::Province;
use crate::tax::calculate_tax;
use chrono::NaiveDate;
use std::fmt;
use tracing::debug;

mod line_item;
mod line_kind;
mod quote_number;

pub use line_item::LineItem;
pub use line_kind::LineKind;
pub use quote_number::QuoteNumber;

pub struct Quote {
    number: QuoteNumber,
    date: NaiveDate,
    customer: Customer,
    equipment: Equipment,
    items: Vec<LineItem>,
}

impl Quote {
    /// The number and the date are handed in, never made here: a quote doesn't
    /// look at the clock or keep a counter (that keeps it easy to test).
    pub fn new(
        number: QuoteNumber,
        date: NaiveDate,
        customer: Customer,
        equipment: Equipment,
    ) -> Self {
        // Logs never include names, phones or emails: only what we need to debug.
        debug!(
            quote_number = %number,
            province = customer.province().label(),
            "quote created"
        );
        Quote {
            number,
            date,
            customer,
            equipment,
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

        debug!(
            kind = item.kind().label(),
            line_total_cents = item.line_total(),
            "line added"
        );
        self.items.push(item);
        Ok(())
    }

    pub fn number(&self) -> &QuoteNumber {
        &self.number
    }

    pub fn customer(&self) -> &Customer {
        &self.customer
    }

    /// The customer's province decides the taxes (one source of truth).
    fn province(&self) -> &Province {
        self.customer.province()
    }

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
        self.labour_total() + self.parts_total() + self.fees_total() + self.credit_total()
    }

    pub fn gst_amount(&self) -> i64 {
        calculate_tax(self.subtotal(), self.province().gst_rate())
    }

    pub fn pst_amount(&self) -> i64 {
        if let Some(pst_rate) = self.province().pst_rate() {
            calculate_tax(self.subtotal(), pst_rate)
        } else {
            0
        }
    }

    pub fn total(&self) -> i64 {
        self.subtotal() + self.gst_amount() + self.pst_amount()
    }
}

/// Shows a quote the way a customer reads it. Because this writes into
/// `f` instead of printing, the same text can go to the terminal, a web
/// page or a PDF later.
impl fmt::Display for Quote {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f)?;
        writeln!(
            f,
            "{} Quote {}: {} {}",
            "=".repeat(5),
            self.number,
            self.province().label(),
            "=".repeat(5)
        )?;
        writeln!(f, "Date: {}", self.date)?;
        writeln!(f, "Customer: {}", self.customer)?;
        writeln!(f, "Contact: {}", self.customer.contact())?;
        writeln!(f, "{}", "-".repeat(20))?;
        writeln!(f, "Equipment: {}", self.equipment)?;

        for item in &self.items {
            writeln!(
                f,
                "[{}] {}: {} x {} = {}",
                item.kind().label(),
                item.description(),
                item.quantity(),
                format_money(item.signed_unit_price_cents()),
                format_money(item.line_total())
            )?;
        }
        writeln!(f, "{}", "-".repeat(20))?;
        writeln!(f, "Labour Total: {}", format_money(self.labour_total()))?;
        writeln!(f, "Parts Total: {}", format_money(self.parts_total()))?;
        writeln!(f, "Fees Total: {}", format_money(self.fees_total()))?;
        writeln!(f, "Credit Total: {}", format_money(self.credit_total()))?;
        writeln!(f, "{}", "-".repeat(20))?;
        writeln!(f, "Subtotal: {}", format_money(self.subtotal()))?;
        writeln!(f, "{}", "-".repeat(20))?;
        writeln!(
            f,
            "GST {}%: {}",
            self.province().gst_rate(),
            format_money(self.gst_amount())
        )?;
        if let Some(pst_rate) = self.province().pst_rate() {
            writeln!(f, "PST {}%: {}", pst_rate, format_money(self.pst_amount()))?;
        }
        write!(f, "Total: {}", format_money(self.total()))
    }
}
