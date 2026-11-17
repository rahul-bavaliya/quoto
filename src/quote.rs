// src/quote.rs

use crate::customer::Customer;
use crate::equipment::Equipment;
use crate::errors::{QuoteError, QuoteResult};
use crate::money::format_money;
use crate::province::Province;
use crate::tax::calculate_tax;
use chrono::{Days, NaiveDate, NaiveDateTime};
use std::fmt;
use tracing::debug;

mod line_item;
mod line_kind;
mod number_machine;
mod quote_number;

pub use line_item::LineItem;
pub use line_kind::LineKind;
pub use number_machine::{InMemoryQuoteNumbers, QuoteNumbers};
pub use quote_number::QuoteNumber;

/// The prices on a quote are good for this many days after the quote date.
const VALID_FOR_DAYS: u64 = 30;

pub struct Quote {
    number: QuoteNumber,
    created_at: NaiveDateTime,
    /// The last day the prices are honoured. Worked out when the quote is created.
    expires_on: NaiveDate,
    /// The day the machine will be ready. Often unknown when quoting (parts on order), so it can be empty.
    promise_date: Option<NaiveDate>,
    customer: Customer,
    equipment: Equipment,
    items: Vec<LineItem>,
}

impl Quote {
    /// The number and the date and time are handed in, never made here: a quote doesn't
    /// look at the clock or keep a counter (that keeps it easy to test).
    pub fn new(
        number: QuoteNumber,
        created_at: NaiveDateTime,
        customer: Customer,
        equipment: Equipment,
    ) -> Self {
        // Logs never include names, phones or emails: only what we need to debug.
        debug!(
            quote_number = %number,
            province = customer.province().label(),
            "quote created"
        );
        // A date so far in the future that it overflows can't happen in real life;
        // if it ever did, the latest possible date is the honest answer.
        let expires_on = created_at
            .date()
            .checked_add_days(Days::new(VALID_FOR_DAYS))
            .unwrap_or(NaiveDate::MAX);
        Quote {
            number,
            created_at,
            expires_on,
            promise_date: None,
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

    pub fn expires_on(&self) -> NaiveDate {
        self.expires_on
    }

    pub fn promise_date(&self) -> Option<NaiveDate> {
        self.promise_date
    }

    /// Changes the last day the prices are honoured. The quote's own date is allowed
    /// (a quote that is good for today only); a day before it is not.
    pub fn set_expiry(&mut self, date: NaiveDate) -> QuoteResult<()> {
        let quote_date = self.created_at.date();
        if date < quote_date {
            return Err(QuoteError::ExpiryBeforeQuoteDate { date, quote_date });
        }
        self.expires_on = date;
        Ok(())
    }

    /// Sets the day the machine will be ready. It can be later than the expiry date:
    /// the expiry is about the prices, the promise is about the work.
    pub fn set_promise_date(&mut self, date: NaiveDate) -> QuoteResult<()> {
        let quote_date = self.created_at.date();
        if date < quote_date {
            return Err(QuoteError::PromiseBeforeQuoteDate { date, quote_date });
        }
        self.promise_date = Some(date);
        Ok(())
    }

    pub fn customer(&self) -> &Customer {
        &self.customer
    }

    pub fn equipment(&self) -> &Equipment {
        &self.equipment
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
        writeln!(f, "Date: {}", self.created_at.format("%Y-%m-%d %H:%M:%S"))?;
        writeln!(f, "Valid until: {}", self.expires_on)?;
        match self.promise_date {
            Some(date) => writeln!(f, "Promise date: {date}")?,
            None => writeln!(f, "Promise date: to be confirmed")?,
        }
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
