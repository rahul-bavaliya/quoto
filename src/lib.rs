// src/lib.rs: the front door of the Quoto library 🚪
//
// Every feature lives in its own folder (`src/<feature>/mod.rs`) with its
// unit tests next to it (`src/<feature>/tests.rs`). Anything marked `pub`
// here can be used by `main.rs`, by the scenario tests in `tests/`, and
// later by the web API.

pub mod errors;
pub mod line_item;
pub mod money;
pub mod province;
pub mod quote;
pub mod tax;

// The short front door: `use quoto::{Quote, LineItem, ...};`
pub use errors::QuoteError;
pub use line_item::{LineItem, LineKind};
pub use money::format_money;
pub use province::Province;
pub use quote::Quote;
