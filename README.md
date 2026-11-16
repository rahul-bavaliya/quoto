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
===== Quote: Saskatchewan =====
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
cargo test     # run all the unit and scenario tests
RUST_LOG=debug cargo run   # same, with a detailed log on stderr
```

## Logging and errors
- **Logging** uses [`tracing`](https://docs.rs/tracing). `quoto::logging::init()` starts it once in `main`. Logs go to **stderr** (the quote text stays on stdout), the level comes from `RUST_LOG` (default `info`), and logs never contain customer details.
- **Errors** are one enum, `QuoteError` (built with `thiserror`). Each error has a readable message, a stable `code()` (like `INVALID_PHONE`), a `kind()` (`InvalidInput` or `BusinessRule`) and an `exit_code()`. `QuoteResult<T>` is the short name for `Result<T, QuoteError>`.
- `main` is the one place where errors stop the program: it logs the code, prints the message, and exits with code 2 (wrong input) or 3 (business rule).

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
├── errors.rs  logging.rs  money.rs  province.rs  samples.rs  tax.rs
├── customer.rs          Customer, plus the pieces it is built from
├── customer/            address, postal_code, contact, phone_number, email
├── equipment.rs         Equipment, plus its pieces
├── equipment/           meter, meter_type, equipment_type, identification
├── quote.rs             Quote, plus its pieces
└── quote/               line_item, line_kind
tests/
└── quote_scenarios.rs   whole-quote scenarios through the public API
```
One type per file, grouped into feature folders (no `mod.rs`). Each file has its unit tests at the bottom.

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
