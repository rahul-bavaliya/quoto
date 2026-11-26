// src/branch/mod.rs: a dealership location. It operates quotes, and its province decides the taxes.
//
// The pieces live in `src/branch/`: `mod.rs` holds the branch model and `table`
// (create, read, update and set the status; never delete). Shared value types
// such as `Address` and `Location` live in `src/common/`.

use crate::common::PhoneNumber;
use crate::common::{Address, Location};
use crate::errors::{AppError, AppResult};
use crate::status::Status;
use uuid::Uuid;

mod table;

pub(crate) use table::BranchTable;
pub use table::{
    branch_by_number, create_branch, get_branch, list_branches, set_branch_status, update_branch,
};

/// One of the dealership's locations. An admin sets branches up; a writer never
/// types one on a quote. A quote is always made at a branch.
/// **A branch is never deleted**: when it is not in use it becomes inactive.
///
/// Changing a branch (rename, move, new phone) means building the new version with
/// `Branch::new(...)` and giving it the old id with `with_id(...)`: the database then
/// updates the row with that id.
#[derive(Debug, PartialEq)]
pub struct Branch {
    /// The database's id. Empty until this branch is loaded from the database:
    /// a new one never makes its own id.
    id: Option<Uuid>,
    /// The branch number people use, e.g. "001". Kept as text so leading zeros survive.
    number: String,
    name: String,
    address: Address,
    phone: PhoneNumber,
    /// Where the branch is on the map. Optional: not every branch has it.
    location: Option<Location>,
    status: Status,
}

impl Branch {
    /// A new branch is active and has no map location yet (`with_location` adds it).
    pub fn new(
        number: String,
        name: String,
        address: Address,
        phone: PhoneNumber,
    ) -> AppResult<Self> {
        if number.trim().is_empty() {
            return Err(AppError::MissingField {
                field: "branch number",
            });
        }
        if name.trim().is_empty() {
            return Err(AppError::MissingField {
                field: "branch name",
            });
        }
        Ok(Branch {
            id: None,
            number,
            name,
            address,
            phone,
            location: None,
            status: Status::default(),
        })
    }

    /// `None` means "not saved yet". The database gives the id; Quoto never makes one.
    pub fn id(&self) -> Option<Uuid> {
        self.id
    }

    /// Attaches the id the database gave. Used when loading from the database.
    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    /// Adds the map location (it is checked when the `Location` is built).
    pub fn with_location(mut self, location: Location) -> Self {
        self.location = Some(location);
        self
    }

    /// Sets the status. Used when loading from the database (a new branch is active).
    pub fn with_status(mut self, status: Status) -> Self {
        self.status = status;
        self
    }

    /// The branch is no longer in use. It is kept (old quotes point at it) but it can't start new quotes.
    pub fn deactivate(&mut self) {
        self.status = Status::Inactive;
    }

    /// The branch is in use again.
    pub fn activate(&mut self) {
        self.status = Status::Active;
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn is_active(&self) -> bool {
        self.status == Status::Active
    }

    pub fn number(&self) -> &str {
        &self.number
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn address(&self) -> &Address {
        &self.address
    }

    pub fn phone(&self) -> &PhoneNumber {
        &self.phone
    }

    pub fn location(&self) -> Option<&Location> {
        self.location.as_ref()
    }
}
