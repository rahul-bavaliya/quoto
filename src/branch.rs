// src/branch.rs: a dealership location. It operates quotes, and its province decides the taxes.

use crate::country::Country;
use crate::customer::{Address, PhoneNumber};
use crate::errors::{QuoteError, QuoteResult};
use crate::province::Province;
use uuid::Uuid;

mod branch_status;
mod location;

pub use branch_status::BranchStatus;
pub use location::Location;

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
    status: BranchStatus,
}

impl Branch {
    /// A new branch is active and has no map location yet (`with_location` adds it).
    pub fn new(
        number: String,
        name: String,
        address: Address,
        phone: PhoneNumber,
    ) -> QuoteResult<Self> {
        if number.trim().is_empty() {
            return Err(QuoteError::MissingField {
                field: "branch number",
            });
        }
        if name.trim().is_empty() {
            return Err(QuoteError::MissingField {
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
            status: BranchStatus::Active,
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
    pub fn with_status(mut self, status: BranchStatus) -> Self {
        self.status = status;
        self
    }

    /// The branch is no longer in use. It is kept (old quotes point at it) but it can't start new quotes.
    pub fn deactivate(&mut self) {
        self.status = BranchStatus::Inactive;
    }

    /// The branch is in use again.
    pub fn activate(&mut self) {
        self.status = BranchStatus::Active;
    }

    pub fn status(&self) -> BranchStatus {
        self.status
    }

    pub fn is_active(&self) -> bool {
        self.status == BranchStatus::Active
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

    /// The branch's province decides the taxes on its quotes (one source of truth).
    pub fn province(&self) -> &Province {
        self.address.province()
    }

    /// The country follows from the province, so the two can never disagree.
    pub fn country(&self) -> Country {
        self.province().country()
    }
}
