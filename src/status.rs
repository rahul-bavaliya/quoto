// src/status.rs: is this record in use? (one status for every kind of record)

use crate::errors::{AppError, AppResult};
use std::fmt;

/// Whether a record is **in use**. Records that must never be deleted (a branch, and later a
/// customer, a machine or a user) are not removed when they are finished with: they become
/// `Inactive`. An inactive record is kept, because old quotes point at it, but it is not offered
/// for new work.
///
/// This is not a *workflow*: where something is in a process (a quote that is pending, accepted
/// or rejected; a repair order that is open or done) is the job of its own enum, because those
/// have more than two steps and different rules. A new record is `Active` (`Status::default()`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Status {
    #[default]
    Active,
    Inactive,
}

impl Status {
    /// The word kept in the database: "active" or "inactive".
    pub fn label(&self) -> &'static str {
        match self {
            Status::Active => "active",
            Status::Inactive => "inactive",
        }
    }

    /// The other way round: reads the word kept in the database. Anything else is refused.
    pub fn from_label(label: &str) -> AppResult<Self> {
        match label {
            "active" => Ok(Status::Active),
            "inactive" => Ok(Status::Inactive),
            other => Err(AppError::InvalidStatus {
                value: other.to_string(),
            }),
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}
