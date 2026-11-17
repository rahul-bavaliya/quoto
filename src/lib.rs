// src/lib.rs: the front door of the Quoto library 🚪
//
// Each feature is a folder (`src/<feature>/`): `mod.rs` holds the main type and
// lists the pieces, one type per file. All tests live in
// `tests/` and use only the public API. Anything marked `pub` here can be
// used by `main.rs`, by those tests, and later by the web API.

pub mod customer;
pub mod equipment;
pub mod equipment_lookup;
pub mod errors;
pub mod logging;
pub mod money;
pub mod province;
pub mod quote;
pub mod samples;
pub mod tax;

// The short front door: `use quoto::{Quote, LineItem, ...};`
pub use customer::{Address, Contact, Customer, Email, PhoneNumber, PostalCode};
pub use equipment::{
    Equipment, EquipmentDraft, EquipmentSource, EquipmentType, Identification, Meter, MeterType,
};
pub use equipment_lookup::{ChainedLookup, EquipmentLookup, InMemoryEquipmentLookup};
pub use errors::{ErrorKind, QuoteError, QuoteResult};
pub use money::format_money;
pub use province::Province;
pub use quote::{InMemoryQuoteNumbers, LineItem, LineKind, Quote, QuoteNumber, QuoteNumbers};
