// tests/database_api: the web API with a real database behind it
//
// Needs a running database, so these are skipped normally (and in CI). Run them by hand:
//
//     docker compose up -d
//     cargo test --test database_api -- --ignored
//
// The API commits each call for real (that is what it does), and nothing can be deleted through
// it. So every test record is made-up and starts with "API-T-" (a branch's number, a customer's
// name), and the tests remove those rows themselves (with SQL, as test housekeeping) before and
// after each test. One file per concern; `support.rs` is what they share.

mod branches;
mod customers;
mod equipment;
mod logging;
mod patch;
mod support;
