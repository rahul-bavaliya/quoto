// src/repair_order.rs: the work done for a customer, written up from an accepted quote
//
// A repair order **points at the accepted quote it came from** instead of copying it. That is safe
// because an accepted quote is final: its lines, its customer, its machine and its taxes can't
// change any more (only its promise date can, and the repair order keeps its own due date). So
// the repair order shows the quote's lines and totals, and has its own facts: its number, its
// status, its due date, the meter reading when the machine came in, and when the work started
// and ended.
//
// One quote makes at most one repair order. The flow only goes forward (see `RepairOrderStatus`).
// Like a quote, a repair order never looks at the clock (the time is handed in) and never makes
// its own id (the database does).

use crate::equipment::{Meter, MeterType};
use crate::errors::{QuoteError, QuoteResult};
use crate::quote::{Quote, QuoteStatus};
use chrono::{NaiveDate, NaiveDateTime};
use tracing::debug;
use uuid::Uuid;

mod notes;
mod number;
mod status;
mod table;

pub use notes::MAX_NOTE_CHARS;
pub use number::RepairOrderNumber;
pub use status::RepairOrderStatus;
pub use table::{
    NewRepairOrder, RepairOrderFilter, create_repair_order, get_repair_order, list_repair_orders,
    next_repair_order_number, save_repair_order,
};
pub(crate) use table::{RepairOrderNumberTable, RepairOrderTable};

pub struct RepairOrder {
    /// The database's id. Empty until this repair order is loaded from the database.
    id: Option<Uuid>,
    /// How many times this repair order was changed in the database (0 when made).
    version: i32,
    number: RepairOrderNumber,
    /// The shop's date and time, as handed in.
    created_at: NaiveDateTime,
    /// When the machine is promised back. Starts as the quote's promise date.
    due_date: Option<NaiveDate>,
    status: RepairOrderStatus,
    /// The meter when the machine came in. Required here (it is optional on a quote).
    meter_reading: f64,
    meter_type: MeterType,
    started_at: Option<NaiveDateTime>,
    closed_at: Option<NaiveDateTime>,
    /// What the customer says is wrong. Optional; the technician's or writer's own words.
    complaint: Option<String>,
    /// What the technician found to be the cause. Optional.
    cause: Option<String>,
    /// What was done to put it right. Optional.
    correction: Option<String>,
    /// The accepted quote this was written up from: its customer, machine, lines and totals.
    quote: Quote,
}

/// Everything a saved repair order is rebuilt from (see `repair_order/table.rs`).
pub(crate) struct SavedRepairOrder {
    pub(crate) id: Uuid,
    pub(crate) version: i32,
    pub(crate) number: RepairOrderNumber,
    pub(crate) created_at: NaiveDateTime,
    pub(crate) due_date: Option<NaiveDate>,
    pub(crate) status: RepairOrderStatus,
    pub(crate) meter_reading: f64,
    pub(crate) meter_type: MeterType,
    pub(crate) started_at: Option<NaiveDateTime>,
    pub(crate) closed_at: Option<NaiveDateTime>,
    pub(crate) complaint: Option<String>,
    pub(crate) cause: Option<String>,
    pub(crate) correction: Option<String>,
    pub(crate) quote: Quote,
}

impl RepairOrder {
    /// Rebuilds a repair order that was saved. The rules that were checked when it was opened are
    /// not run again: it is what it was that day.
    pub(crate) fn restore(saved: SavedRepairOrder) -> RepairOrder {
        RepairOrder {
            id: Some(saved.id),
            version: saved.version,
            number: saved.number,
            created_at: saved.created_at,
            due_date: saved.due_date,
            status: saved.status,
            meter_reading: saved.meter_reading,
            meter_type: saved.meter_type,
            started_at: saved.started_at,
            closed_at: saved.closed_at,
            complaint: saved.complaint,
            cause: saved.cause,
            correction: saved.correction,
            quote: saved.quote,
        }
    }

    /// Writes up a repair order from an accepted quote. The number and the date and time are
    /// handed in, never made here.
    ///
    /// - The quote must be saved and **accepted** (an accepted quote always has a customer and a
    ///   machine).
    /// - The **meter reading is required**: `meter_reading` is what the writer read on the
    ///   machine today; if none is given the quote's own reading is used; with neither it is
    ///   refused (`MeterReadingRequired`). A reading must be a number that is zero or more.
    /// - The due date starts as the quote's promise date (it can be empty: to be confirmed).
    pub fn open(
        number: RepairOrderNumber,
        created_at: NaiveDateTime,
        quote: Quote,
        meter_reading: Option<f64>,
    ) -> QuoteResult<Self> {
        if quote.id().is_none() {
            return Err(QuoteError::QuoteNotSaved);
        }
        if !matches!(quote.status(), QuoteStatus::Accepted) {
            return Err(QuoteError::QuoteNotAccepted);
        }
        let machine = quote
            .equipment()
            .ok_or(QuoteError::QuoteIncomplete { missing: "machine" })?;
        let meter_type = machine.meter().meter_type();
        let reading = meter_reading
            .or(machine.meter().reading())
            .ok_or(QuoteError::MeterReadingRequired)?;
        // The same check as every other meter reading: a real number, zero or more.
        Meter::new(Some(reading), meter_type)?;
        debug!(repair_order_number = %number, "repair order opened");
        Ok(RepairOrder {
            id: None,
            version: 0,
            number,
            created_at,
            due_date: quote.promise_date(),
            status: RepairOrderStatus::Open,
            meter_reading: reading,
            meter_type,
            started_at: None,
            closed_at: None,
            complaint: None,
            cause: None,
            correction: None,
            quote,
        })
    }

    /// `None` until the repair order is loaded from the database.
    pub fn id(&self) -> Option<Uuid> {
        self.id
    }

    /// How many times this repair order has been changed (0 when made).
    pub fn version(&self) -> i32 {
        self.version
    }

    pub fn number(&self) -> &RepairOrderNumber {
        &self.number
    }

    pub fn created_at(&self) -> NaiveDateTime {
        self.created_at
    }

    pub fn due_date(&self) -> Option<NaiveDate> {
        self.due_date
    }

    pub fn status(&self) -> &RepairOrderStatus {
        &self.status
    }

    /// The meter when the machine came in.
    pub fn meter_reading(&self) -> f64 {
        self.meter_reading
    }

    pub fn meter_type(&self) -> MeterType {
        self.meter_type
    }

    /// When the work started (empty while the repair order is still open).
    pub fn started_at(&self) -> Option<NaiveDateTime> {
        self.started_at
    }

    /// When it was completed or cancelled.
    pub fn closed_at(&self) -> Option<NaiveDateTime> {
        self.closed_at
    }

    /// The accepted quote: its customer, machine, lines, totals and taxes.
    pub fn quote(&self) -> &Quote {
        &self.quote
    }

    /// The work begins. Only an open repair order can be started.
    pub fn start(&mut self, at: NaiveDateTime) -> QuoteResult<()> {
        if self.status != RepairOrderStatus::Open {
            return Err(QuoteError::RepairOrderNotOpen);
        }
        self.status = RepairOrderStatus::InProgress;
        self.started_at = Some(at);
        debug!(repair_order_number = %self.number, "repair order started");
        Ok(())
    }

    /// The work is done. Only a repair order that is in progress can be completed. Final.
    pub fn complete(&mut self, at: NaiveDateTime) -> QuoteResult<()> {
        if self.status != RepairOrderStatus::InProgress {
            return Err(QuoteError::RepairOrderNotInProgress);
        }
        self.status = RepairOrderStatus::Completed;
        self.closed_at = Some(at);
        debug!(repair_order_number = %self.number, "repair order completed");
        Ok(())
    }

    /// Stopped before it was completed, and why. An open or in-progress repair order can be
    /// cancelled; a completed or cancelled one can't. Final. The reason is somebody's own words,
    /// so it is never logged.
    pub fn cancel(&mut self, reason: &str, at: NaiveDateTime) -> QuoteResult<()> {
        self.ensure_not_closed()?;
        self.status = RepairOrderStatus::cancelled(reason)?;
        self.closed_at = Some(at);
        debug!(repair_order_number = %self.number, "repair order cancelled");
        Ok(())
    }

    /// Changes the due date (or clears it: "to be confirmed"). A date can't be before the day the
    /// repair order was opened. A completed or cancelled repair order can't be changed.
    pub fn set_due_date(&mut self, date: Option<NaiveDate>) -> QuoteResult<()> {
        self.ensure_not_closed()?;
        if let Some(date) = date
            && date < self.created_at.date()
        {
            return Err(QuoteError::DueDateBeforeOpened);
        }
        self.due_date = date;
        Ok(())
    }

    pub fn complaint(&self) -> Option<&str> {
        self.complaint.as_deref()
    }

    pub fn cause(&self) -> Option<&str> {
        self.cause.as_deref()
    }

    pub fn correction(&self) -> Option<&str> {
        self.correction.as_deref()
    }

    /// Writes (or clears, with `None` or blank text) what the customer **complained** about.
    /// Optional and free text; see `notes::clean_note` for the rules. Only an open or
    /// in-progress repair order can change. The words are somebody's own: never logged.
    pub fn set_complaint(&mut self, text: Option<&str>) -> QuoteResult<()> {
        self.ensure_not_closed()?;
        self.complaint = notes::clean_note("complaint", text)?;
        Ok(())
    }

    /// Writes (or clears) the **cause** the technician found. Same rules as the complaint.
    pub fn set_cause(&mut self, text: Option<&str>) -> QuoteResult<()> {
        self.ensure_not_closed()?;
        self.cause = notes::clean_note("cause", text)?;
        Ok(())
    }

    /// Writes (or clears) the **correction**: what was done to fix it. Same rules as the complaint.
    pub fn set_correction(&mut self, text: Option<&str>) -> QuoteResult<()> {
        self.ensure_not_closed()?;
        self.correction = notes::clean_note("correction", text)?;
        Ok(())
    }

    fn ensure_not_closed(&self) -> QuoteResult<()> {
        if self.status.is_closed() {
            return Err(QuoteError::RepairOrderClosed);
        }
        Ok(())
    }
}
