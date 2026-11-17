// src/branch/branch_status.rs: is the branch open for business?

use std::fmt;

/// A branch is never deleted: old quotes point at it. When a branch is not in use,
/// it becomes `Inactive` and can no longer start new quotes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BranchStatus {
    Active,
    Inactive,
}

impl BranchStatus {
    /// The word kept in the database: "active" or "inactive".
    pub fn label(&self) -> &'static str {
        match self {
            BranchStatus::Active => "active",
            BranchStatus::Inactive => "inactive",
        }
    }
}

impl fmt::Display for BranchStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}
