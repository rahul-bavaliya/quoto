// src/lib.rs: the front door of the Quoto library 🚪
//
// Each feature is a file named after it (`src/<feature>.rs`) that holds the main
// type and lists the pieces; a feature with pieces also has a folder of the same
// name (`src/<feature>/`), one type per file. All tests live in
// `tests/` and use only the public API. Anything marked `pub` here can be
// used by `main.rs`, by those tests, and later by the web API.

pub mod api;
pub mod branch;
pub mod country;
pub mod customer;
pub mod database;
pub mod errors;
pub mod province;
pub mod secret;
pub mod status;

// The short front door: `use quoto::{Quote, LineItem, ...};`
pub use branch::{
    Branch, Location, branch_by_number, create_branch, get_branch, list_branches,
    set_branch_status, update_branch,
};
pub use country::Country;
pub use customer::{
    Address, PhoneNumber, PostalCode,
};
pub use database::DatabaseSettings;
pub use errors::{ErrorKind, QuoteError, QuoteResult};
pub use province::{
    Province,
};
pub use secret::Secret;
pub use status::Status;
