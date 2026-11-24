// src/lib.rs: the front door of the Quoto library 🚪
//
// Each feature has a folder under `src/<feature>/`, with `mod.rs` holding the
// main type and one type per supporting file. All tests live in
// `tests/` and use only the public API. Anything marked `pub` here can be
// used by `main.rs`, by those tests, and later by the web API.

pub mod api;
#[path = "branch/branch.rs"]
pub mod branch;
pub mod common;
pub mod country;
pub mod customer;
pub mod database;
pub mod errors;
pub mod province;
pub mod secret;
pub mod status;

// The short front door: `use quoto::{Quote, LineItem, ...};`
pub use branch::{
    Branch, branch_by_number, create_branch, get_branch, list_branches, set_branch_status,
    update_branch,
};
pub use common::{Address, Location, PhoneNumber, PostalCode};
pub use country::Country;
pub use database::DatabaseSettings;
pub use errors::{AppError, AppResult, ErrorKind};
pub use province::Province;
pub use secret::Secret;
pub use status::Status;
