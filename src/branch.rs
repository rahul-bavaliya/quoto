// src/branch.rs: a dealership location. It operates quotes, and its province decides the taxes.

use crate::customer::{Address, PhoneNumber};
use crate::errors::{QuoteError, QuoteResult};
use crate::province::Province;
use uuid::Uuid;

/// One of the dealership's locations. An admin sets branches up; a writer never
/// types one on a quote. A quote is always made at a branch.
#[derive(Debug, PartialEq)]
pub struct Branch {
    /// The database's id. Empty until this branch is loaded from the database:
    /// a new one never makes its own id.
    id: Option<Uuid>,
    code: String,
    name: String,
    address: Address,
    phone: PhoneNumber,
}

impl Branch {
    pub fn new(
        code: String,
        name: String,
        address: Address,
        phone: PhoneNumber,
    ) -> QuoteResult<Self> {
        if code.trim().is_empty() {
            return Err(QuoteError::MissingField {
                field: "branch code",
            });
        }
        if name.trim().is_empty() {
            return Err(QuoteError::MissingField {
                field: "branch name",
            });
        }
        Ok(Branch {
            id: None,
            code,
            name,
            address,
            phone,
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

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn phone(&self) -> &PhoneNumber {
        &self.phone
    }

    /// The branch's province decides the taxes on its quotes (one source of truth).
    pub fn province(&self) -> &Province {
        self.address.province()
    }
}
