// src/quote/quote_status.rs: where a quote stands with the customer

use crate::errors::{QuoteError, QuoteResult};
use std::fmt;

/// The longest rejection reason we keep, counted in characters (not bytes).
const MAX_REASON_CHARS: usize = 500;

/// Where a quote stands with the customer. The reason lives inside `Rejected`,
/// so a rejection without a reason, or a reason on an accepted quote, can't exist.
///
/// "Expired" is not a status: it depends on today's date, so it is worked out
/// when asked (`Quote::is_expired`) instead of being stored, where it would go stale.
#[derive(Debug, Clone, PartialEq)]
pub enum QuoteStatus {
    /// Waiting for the customer to decide.
    Pending,
    Accepted,
    /// The customer said no, and why.
    Rejected {
        reason: String,
    },
}

impl QuoteStatus {
    /// A rejection with a checked reason: not blank, at most 500 characters.
    /// The spaces around the reason are dropped.
    pub(super) fn rejected(reason: &str) -> QuoteResult<Self> {
        let reason = reason.trim();
        if reason.is_empty() {
            return Err(QuoteError::MissingField {
                field: "rejection reason",
            });
        }
        if reason.chars().count() > MAX_REASON_CHARS {
            return Err(QuoteError::RejectionReasonTooLong {
                max: MAX_REASON_CHARS,
            });
        }
        Ok(QuoteStatus::Rejected {
            reason: reason.to_string(),
        })
    }

    /// One word for logs. It never includes the reason: that is the customer's own
    /// text and may hold personal details.
    pub fn label(&self) -> &'static str {
        match self {
            QuoteStatus::Pending => "pending",
            QuoteStatus::Accepted => "accepted",
            QuoteStatus::Rejected { .. } => "rejected",
        }
    }
}

impl fmt::Display for QuoteStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QuoteStatus::Rejected { reason } => write!(f, "rejected ({reason})"),
            other => write!(f, "{}", other.label()),
        }
    }
}
