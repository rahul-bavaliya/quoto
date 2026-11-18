// tests/api: the web API's key check, input checks, docs and settings (public API only)
//
// These calls go straight into the router (no network, no database): every one of them is
// answered before the database would be touched. One file per concern; `support.rs` is what
// they share. (The calls that do reach a database are in `tests/database_api/`.)

mod access;
mod branches;
mod customers;
mod docs;
mod equipment;
mod patch;
mod settings;
mod support;
