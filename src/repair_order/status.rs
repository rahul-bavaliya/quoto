// src/repair_order/status.rs: where a repair order stands in the shop

use crate::errors::{QuoteError, QuoteResult};

/// The longest cancellation reason we keep, counted in characters (not bytes).
const MAX_REASON_CHARS: usize = 500;

/// Where a repair order stands. The flow only goes forward:
///
/// ```text
/// Open ──start──> InProgress ──complete──> Completed
///   └──────────cancel──────────┴────────> Cancelled { reason }
/// ```
///
/// Completed and Cancelled are final. The reason lives inside `Cancelled`, so a cancellation
/// without a reason, or a reason on a repair order that was not cancelled, can't exist.
#[derive(Debug, Clone, PartialEq)]
pub enum RepairOrderStatus {
    /// Written up; the work has not started.
    Open,
    /// The work is being done.
    InProgress,
    /// The work is done. Final.
    Completed,
    /// Stopped before it was completed, and why. Final.
    Cancelled { reason: String },
}

impl RepairOrderStatus {
    /// A cancellation with a checked reason: not blank, at most 500 characters. The spaces around
    /// the reason are dropped.
    pub(super) fn cancelled(reason: &str) -> QuoteResult<Self> {
        let reason = reason.trim();
        if reason.is_empty() {
            return Err(QuoteError::MissingField {
                field: "cancellation reason",
            });
        }
        if reason.chars().count() > MAX_REASON_CHARS {
            return Err(QuoteError::CancellationReasonTooLong {
                max: MAX_REASON_CHARS,
            });
        }
        Ok(RepairOrderStatus::Cancelled {
            reason: reason.to_string(),
        })
    }

    /// One word for logs, the database and the API. Never includes the reason: that is
    /// somebody's own text and may hold personal details.
    pub fn label(&self) -> &'static str {
        match self {
            RepairOrderStatus::Open => "open",
            RepairOrderStatus::InProgress => "in_progress",
            RepairOrderStatus::Completed => "completed",
            RepairOrderStatus::Cancelled { .. } => "cancelled",
        }
    }

    /// Completed and cancelled repair orders can't change any more.
    pub fn is_closed(&self) -> bool {
        matches!(
            self,
            RepairOrderStatus::Completed | RepairOrderStatus::Cancelled { .. }
        )
    }
}
