// src/lib.rs: the front door of the Quoto library 🚪
//
// Each feature is a file (`src/<feature>.rs`) plus a folder of the pieces it
// is built from (`src/<feature>/`), one type per file, with unit tests at the
// bottom in a `#[cfg(test)] mod tests`. Anything marked `pub`
// here can be used by `main.rs`, by the scenario tests in `tests/`, and
// later by the web API.

pub mod customer;
pub mod equipment;
pub mod errors;
pub mod money;
pub mod province;
pub mod quote;
pub mod tax;

// The short front door: `use quoto::{Quote, LineItem, ...};`
pub use customer::{Address, Contact, Customer, Email, PhoneNumber, PostalCode};
pub use equipment::{Equipment, EquipmentType, Identification, Meter, MeterType};
pub use errors::QuoteError;
pub use money::format_money;
pub use province::Province;
pub use quote::{LineItem, LineKind, Quote};
