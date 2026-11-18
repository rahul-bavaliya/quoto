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
pub mod equipment;
pub mod equipment_lookup;
pub mod errors;
pub mod logging;
pub mod money;
pub mod province;
pub mod quote;
pub mod samples;
pub mod secret;
pub mod status;

// The short front door: `use quoto::{Quote, LineItem, ...};`
pub use branch::{
    Branch, Location, branch_by_number, create_branch, get_branch, list_branches,
    set_branch_status, update_branch,
};
pub use country::Country;
pub use customer::{Address, Contact, Customer, Email, PhoneNumber, PostalCode};
pub use database::DatabaseSettings;
pub use equipment::{
    Equipment, EquipmentDraft, EquipmentSource, EquipmentType, Identification, Meter, MeterType,
};
pub use equipment_lookup::{ChainedLookup, EquipmentLookup, InMemoryEquipmentLookup, NhtsaLookup};
pub use errors::{ErrorKind, QuoteError, QuoteResult};
pub use money::format_money;
pub use province::{
    Province, TaxRate, TaxRates, all_rates, calculate_tax, change_rates, rates_for, seed_provinces,
};
pub use quote::{
    InMemoryQuoteNumbers, LineItem, LineKind, Quote, QuoteNumber, QuoteNumbers, QuoteStatus,
};
pub use secret::Secret;
pub use status::Status;
