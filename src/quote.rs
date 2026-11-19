// src/quote.rs

use crate::branch::Branch;
use crate::customer::Customer;
use crate::equipment::Equipment;
use crate::errors::{QuoteError, QuoteResult};
use crate::province::Province;
use chrono::{Days, NaiveDate, NaiveDateTime};
use tracing::debug;
use uuid::Uuid;

mod line_item;
mod line_kind;
mod number_machine;
mod print;
mod quote_number;
mod quote_status;
mod table;
mod totals;

pub use line_item::LineItem;
pub use line_kind::LineKind;
pub use number_machine::{InMemoryQuoteNumbers, QuoteNumbers};
pub use quote_number::QuoteNumber;
pub use quote_status::QuoteStatus;
pub use table::{
    NewQuote, QuoteFilter, create_quote, get_quote, list_quotes, next_quote_number, save_quote,
};
pub(crate) use table::{QuoteLineTable, QuoteNumberTable, QuoteTable};

/// The prices on a quote are good for this many days after the quote date.
const VALID_FOR_DAYS: u64 = 30;

/// Limits that keep a quote readable and its totals from overflowing.
pub const MAX_QUANTITY: f64 = 1_000_000.0;
pub const MAX_UNIT_PRICE_CENTS: i64 = 100_000_000;
pub const MAX_DESCRIPTION_CHARS: usize = 200;
pub const MAX_LINES: usize = 200;

pub struct Quote {
    /// The database's id. Empty until this quote is loaded from the database:
    /// a new one never makes its own id.
    id: Option<Uuid>,
    /// How many times this quote has been changed in the database (0 when made). A change is
    /// accepted only if it was made from the latest version: see `save_quote`.
    version: i32,
    number: QuoteNumber,
    created_at: NaiveDateTime,
    /// The last day the prices are honoured. Worked out when the quote is created.
    expires_on: NaiveDate,
    /// The day the machine will be ready. Often unknown when quoting (parts on order), so it can be empty.
    promise_date: Option<NaiveDate>,
    status: QuoteStatus,
    /// The location that operates this quote. Always known; its province decides the taxes.
    branch: Branch,
    /// Empty on a quick quote (a walk-in or a phone call). Needed before the quote can be accepted.
    customer: Option<Customer>,
    /// Empty until the writer adds the machine. A machine always belongs to the quote's customer.
    equipment: Option<Equipment>,
    items: Vec<LineItem>,
}

/// Everything a saved quote is rebuilt from (see `quote/table.rs`).
pub(crate) struct SavedQuote {
    pub(crate) id: Uuid,
    pub(crate) version: i32,
    pub(crate) number: QuoteNumber,
    pub(crate) created_at: NaiveDateTime,
    pub(crate) expires_on: NaiveDate,
    pub(crate) promise_date: Option<NaiveDate>,
    pub(crate) status: QuoteStatus,
    pub(crate) branch: Branch,
    pub(crate) customer: Option<Customer>,
    pub(crate) equipment: Option<Equipment>,
    pub(crate) items: Vec<LineItem>,
}

impl Quote {
    /// Rebuilds a quote that was saved. The rules that were checked when it was made are not run
    /// again: the quote is what it was that day (its branch may be inactive now, its customer
    /// may have moved). Every part was still checked as it was read.
    pub(crate) fn restore(saved: SavedQuote) -> Quote {
        Quote {
            id: None,
            version: saved.version,
            number: saved.number,
            created_at: saved.created_at,
            expires_on: saved.expires_on,
            promise_date: saved.promise_date,
            status: saved.status,
            branch: saved.branch,
            customer: saved.customer,
            equipment: saved.equipment,
            items: saved.items,
        }
        .with_id(saved.id)
    }

    /// The number and the date and time are handed in, never made here: a quote doesn't
    /// look at the clock or keep a counter (that keeps it easy to test).
    /// A new quote starts with just its branch (a quick quote). The customer and the
    /// machine are added with `attach_customer` and `attach_equipment`.
    /// An inactive branch can't start new quotes (its old quotes stay as they are).
    pub fn new(
        number: QuoteNumber,
        created_at: NaiveDateTime,
        branch: Branch,
    ) -> QuoteResult<Self> {
        if !branch.is_active() {
            return Err(QuoteError::BranchInactive);
        }
        // Logs never include names, phones or emails: only what we need to debug.
        debug!(
            quote_number = %number,
            province = branch.province().label(),
            "quote created"
        );
        // A date so far in the future that it overflows can't happen in real life;
        // if it ever did, the latest possible date is the honest answer.
        let expires_on = created_at
            .date()
            .checked_add_days(Days::new(VALID_FOR_DAYS))
            .unwrap_or(NaiveDate::MAX);
        Ok(Quote {
            id: None,
            version: 0,
            number,
            created_at,
            expires_on,
            promise_date: None,
            status: QuoteStatus::Pending,
            branch,
            customer: None,
            equipment: None,
            items: Vec::new(),
        })
    }

    /// Adds a line to the quote. A credit is refused if it would push
    /// the subtotal below zero (the dealer would owe the customer).
    /// Only a pending quote can change: once the customer has decided, the
    /// price they decided on must not move (a rejected quote can be reopened).
    pub fn add_item(&mut self, mut item: LineItem) -> Result<(), QuoteError> {
        self.ensure_pending()?;
        if self.items.len() >= MAX_LINES {
            return Err(QuoteError::TooManyLines { max: MAX_LINES });
        }
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
        // The quote is the one place that says which quote a line belongs to.
        if let Some(id) = self.id {
            item.belong_to(id);
        }
        self.items.push(item);
        Ok(())
    }

    /// `None` means "not saved yet". The database gives the id; Quoto never makes one.
    pub fn id(&self) -> Option<Uuid> {
        self.id
    }

    /// How many times the saved quote has been changed (0 for a new one). `save_quote` accepts
    /// a change only if it was made from the latest version.
    pub fn version(&self) -> i32 {
        self.version
    }

    /// Attaches the id the database gave. Used when loading from the database.
    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        // Lines already on the quote now know which quote they belong to.
        for item in &mut self.items {
            item.belong_to(id);
        }
        self
    }

    /// The lines on the quote, in the order they were added.
    pub fn items(&self) -> &[LineItem] {
        &self.items
    }

    pub fn number(&self) -> &QuoteNumber {
        &self.number
    }

    /// The shop's date and time the quote was made (as it was handed in).
    pub fn created_at(&self) -> NaiveDateTime {
        self.created_at
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

    pub fn status(&self) -> &QuoteStatus {
        &self.status
    }

    /// Is the quote past its expiry date? Today's date is handed in (a quote never
    /// looks at the clock), so a quote loaded from the database later is judged by
    /// the day it is asked on. The expiry date itself is still valid.
    /// Only a pending quote can expire: an accepted quote keeps its price.
    pub fn is_expired(&self, today: NaiveDate) -> bool {
        matches!(self.status, QuoteStatus::Pending) && today > self.expires_on
    }

    /// The customer accepts. Refused if the quote has already been decided,
    /// if it has no customer or machine yet (a repair order needs both),
    /// or if its prices have expired (extend the expiry date first).
    pub fn accept(&mut self, today: NaiveDate) -> QuoteResult<()> {
        self.ensure_pending()?;
        if self.customer.is_none() {
            return Err(QuoteError::QuoteIncomplete {
                missing: "customer",
            });
        }
        if self.equipment.is_none() {
            return Err(QuoteError::QuoteIncomplete {
                missing: "equipment",
            });
        }
        if self.is_expired(today) {
            return Err(QuoteError::QuoteExpired {
                valid_until: self.expires_on,
            });
        }
        self.status = QuoteStatus::Accepted;
        debug!(quote_number = %self.number, "quote accepted");
        Ok(())
    }

    /// The customer says no, and why. An expired quote can still be rejected.
    /// The reason is the customer's own words, so it is never logged.
    pub fn reject(&mut self, reason: &str) -> QuoteResult<()> {
        self.ensure_pending()?;
        self.status = QuoteStatus::rejected(reason)?;
        debug!(quote_number = %self.number, "quote rejected");
        Ok(())
    }

    /// A rejected quote goes back to pending (the customer came back). The old reason is dropped.
    /// An accepted quote is final: it becomes a repair order later.
    pub fn reopen(&mut self) -> QuoteResult<()> {
        if !matches!(self.status, QuoteStatus::Rejected { .. }) {
            return Err(QuoteError::NotRejected);
        }
        self.status = QuoteStatus::Pending;
        debug!(quote_number = %self.number, "quote reopened");
        Ok(())
    }

    /// Only a pending quote can be decided or changed.
    fn ensure_pending(&self) -> QuoteResult<()> {
        match &self.status {
            QuoteStatus::Pending => Ok(()),
            decided => Err(QuoteError::AlreadyDecided {
                status: decided.label(),
            }),
        }
    }

    pub fn branch(&self) -> &Branch {
        &self.branch
    }

    pub fn customer(&self) -> Option<&Customer> {
        self.customer.as_ref()
    }

    pub fn equipment(&self) -> Option<&Equipment> {
        self.equipment.as_ref()
    }

    /// Adds the customer to a pending quote. A quote has one customer: adding a second is refused.
    pub fn attach_customer(&mut self, customer: Customer) -> QuoteResult<()> {
        self.ensure_pending()?;
        if self.customer.is_some() {
            return Err(QuoteError::AlreadyAttached { what: "customer" });
        }
        if !customer.is_active() {
            return Err(QuoteError::CustomerInactive);
        }
        self.customer = Some(customer);
        debug!(quote_number = %self.number, "customer added");
        Ok(())
    }

    /// Adds the machine to a pending quote. The customer comes first, and the machine
    /// must be theirs: when both are known to the database and the owner is someone
    /// else, it is refused. (A machine not saved yet has no owner to compare.)
    pub fn attach_equipment(&mut self, equipment: Equipment) -> QuoteResult<()> {
        self.ensure_pending()?;
        if self.equipment.is_some() {
            return Err(QuoteError::AlreadyAttached { what: "equipment" });
        }
        if !equipment.is_active() {
            return Err(QuoteError::EquipmentInactive);
        }
        let Some(customer) = &self.customer else {
            return Err(QuoteError::EquipmentNeedsCustomer);
        };
        if let (Some(owner), Some(customer_id)) = (equipment.customer_id(), customer.id())
            && owner != customer_id
        {
            return Err(QuoteError::EquipmentNotOwnedByCustomer);
        }
        self.equipment = Some(equipment);
        debug!(quote_number = %self.number, "equipment added");
        Ok(())
    }

    /// The branch's province decides the taxes (one source of truth).
    pub fn province(&self) -> &Province {
        self.branch.province()
    }
}
