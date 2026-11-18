// src/quote/print.rs: a quote the way a customer reads it (the text; `totals.rs` has the math)

use super::Quote;
use crate::money::format_money;
use std::fmt;

/// Shows a quote the way a customer reads it. Because this writes into
/// `f` instead of printing, the same text can go to the terminal, a web
/// page or a PDF later.
impl fmt::Display for Quote {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f)?;
        writeln!(
            f,
            "===== Quote {}: {} =====",
            self.number,
            self.province().label()
        )?;
        writeln!(f, "Branch: {}", self.branch.name())?;
        writeln!(f, "Date: {}", self.created_at.format("%Y-%m-%d %H:%M:%S"))?;
        writeln!(f, "Valid until: {}", self.expires_on)?;
        match self.promise_date {
            Some(date) => writeln!(f, "Promise date: {date}")?,
            None => writeln!(f, "Promise date: to be confirmed")?,
        }
        writeln!(f, "Status: {}", self.status)?;
        match &self.customer {
            Some(customer) => {
                writeln!(f, "Customer: {customer}")?;
                writeln!(f, "Contact: {}", customer.contact())?;
            }
            None => writeln!(f, "Customer: to be added")?,
        }
        writeln!(f, "{}", "-".repeat(20))?;
        match &self.equipment {
            Some(equipment) => writeln!(f, "Equipment: {equipment}")?,
            None => writeln!(f, "Equipment: to be added")?,
        }

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
