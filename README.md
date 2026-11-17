# Quoto 🦀

**Quoto** is a quoting and repair-order system for equipment and vehicle service shops, built from scratch in Rust.

It's also a learning project: every feature is built one small step at a time, with tests guarding the math.

## What it can do today
- Build a **quote** from line items: **labour**, **parts**, **fees** and **credits** (goodwill, discounts)
- **Saskatchewan** and **Alberta** taxes: GST and PST are calculated and shown **separately** (Alberta has no PST)
- **Safe money:** every amount is stored as whole **cents** (`i64`), so there are no floating-point crumbs
- **Credits come off before tax**, and a credit can never push a quote below $0
- **Validation:** bad lines (zero quantity, negative price) are refused with a clear `QuoteError`

```
===== Quote Q-2026-00000001: Saskatchewan =====
Date: 2026-11-17 10:58:47
[PART] Oil Filter: 1 x $25.00 = $25.00
[LABOUR] Oil Change Labour: 0.5 x $160.00 = $80.00
...
[CREDIT] Goodwill - 3 day wait: 1 x -$50.00 = -$50.00
--------------------
Subtotal: $123.00
GST 5%: $6.15
PST 6%: $7.38
Total: $136.53
```

## Run it
```bash
cargo run      # print the demo quotes
cargo run --example nhtsa_lookup -- <VIN>   # try the real NHTSA lookup (needs the internet)
cargo test     # run all the unit and scenario tests
RUST_LOG=debug cargo run   # same, with a detailed log on stderr
```

## Looking up equipment
Type a serial number and a **lookup** fills in the make, model, year and kind of machine. The service writer then adds the plate and the meter reading, and `Equipment::from_draft` checks everything.
- `EquipmentLookup` is the promise: `lookup(serial)` gives `Ok(Some(draft))` (found), `Ok(None)` (not known) or `Err` (the service is down). In every case but "found", the details are typed in by hand.
- **`NhtsaLookup`** asks the US government's free [vPIC](https://vpic.nhtsa.dot.gov/api/) service. It knows road vehicles with a 17-character VIN and only trusts a clean decode. Only the VIN leaves your computer.
- **`InMemoryEquipmentLookup`** is a small catalog of made-up machines for the demo and the tests (it knows a tractor, which NHTSA does not).
- **`ChainedLookup`** asks several lookups in order, so road vehicles and heavy equipment can each have their own. Equipment remembers its `EquipmentSource` (entered by hand, or looked up with whom).
- Tests never touch the internet. To try the real service by hand: `cargo run --example nhtsa_lookup -- 1HGCM82633A004352`

## Logging and errors
- **Logging** uses [`tracing`](https://docs.rs/tracing). `quoto::logging::init()` starts it once in `main`. Logs go to **stderr** (the quote text stays on stdout), the level comes from `RUST_LOG` (default `info`), and logs never contain customer details.
- **Errors** are one enum, `QuoteError` (built with `thiserror`). Each error has a readable message, a stable `code()` (like `INVALID_PHONE`), a `kind()` (`InvalidInput`, `BusinessRule` or `ServiceUnavailable`) and an `exit_code()`. `QuoteResult<T>` is the short name for `Result<T, QuoteError>`.
- `main` is the one place where errors stop the program: it logs the code, prints the message, and exits with code 2 (wrong input), 3 (business rule) or 4 (an outside service is down).

Before every push:
```bash
cargo fmt
cargo clippy --all-targets
cargo test
```
GitHub Actions runs the same checks automatically on every push and pull request.

## Project layout
```
src/
├── main.rs              demo program (short: it uses quoto::samples)
├── lib.rs               the library's front door
├── errors/  logging/  money/  province/  samples/  tax/     each is a folder with a mod.rs
├── customer/            mod.rs (Customer) + address, postal_code, contact, phone_number, email
├── equipment/           mod.rs (Equipment) + meter, meter_type, equipment_type, identification,
│                        equipment_draft, equipment_source
├── equipment_lookup/    mod.rs (EquipmentLookup) + in_memory, chain, nhtsa
└── quote/               mod.rs (Quote) + line_item, line_kind, quote_number, number_machine
examples/
└── nhtsa_lookup.rs      try the lookups by hand (needs the internet)
tests/                   one file per feature, plus quote_scenarios.rs and demo_program.rs
```
One type per file, grouped into feature folders (each folder's `mod.rs` holds the main type and lists its pieces). All tests live in `tests/` (one file per feature) and only use the public API.

## Roadmap
1. ✅ Toy calculator
2. ✅ Real shapes: structs, enums, `Vec`, `Option`
3. ✅ Safe money and errors: cents, `Result`, custom errors, tests
4. ⬜ Save quotes in a database (tax rates as data, with effective dates)
5. ⬜ A web API
6. ⬜ Quote → Repair Order
7. ⬜ Screens for service writers
8. ⬜ Invoices and PDFs
9. ⬜ Users and roles
10. ⬜ Ship it
