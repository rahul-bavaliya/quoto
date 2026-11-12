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
```

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
├── main.rs              demo program (uses the quoto library)
├── lib.rs               the library's front door
├── <feature>/mod.rs     the feature's code
├── <feature>/tests.rs   its unit tests
tests/
└── quote_scenarios.rs   whole-quote scenarios through the public API
```
Features: `errors`, `line_item`, `money`, `province`, `quote`, `tax`.

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
