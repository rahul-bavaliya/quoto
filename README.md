# Quoto 🦀

**Quoto** is a quoting and repair-order system for equipment and vehicle service shops, built from scratch in Rust.

It's also a learning project: every feature is built one small step at a time, with tests guarding the math.

## What it can do today
- Build a **quote** from line items: **labour**, **parts**, **fees** and **credits** (goodwill, discounts)
- **Saskatchewan** and **Alberta** taxes: GST and PST are calculated and shown **separately** (Alberta has no PST)
- **Safe money:** every amount is stored as whole **cents** (`i64`), so there are no floating-point crumbs
- **Credits come off before tax**, and a credit can never push a quote below $0
- **Dates:** every quote shows its **expiry date** (quote date + 30 days; can be changed, never before the quote date) and a **promise date** (when the machine will be ready; optional, shows "to be confirmed" until it is set)
- **Status:** a quote is **pending**, **accepted**, or **rejected** (a rejection must say why, up to 500 characters). Only a pending quote can change or be decided; an accepted quote is final; a rejected one can be reopened. Whether a quote is **expired** is worked out from today's date (`quote.is_expired(today)`), and an expired quote can't be accepted until its expiry date is extended
- **Branch and who a quote is for:** every quote is made at a **branch** (`Branch`: number, name, address with a second line, city, province, postal code, phone, an optional map location, and a status). The country follows from the province (`Province::country()`), so a branch can't say "Alberta, Germany". **A branch is never deleted**: when it is not in use it is made inactive (`deactivate()`), and an inactive branch can't start new quotes (`Quote::new` refuses it); its old quotes are untouched. The **branch's province decides the taxes**, not the customer's address. A new quote has only its branch (a *quick quote*, for a walk-in or a phone call); the customer and the machine are added later with `attach_customer(...)` and `attach_equipment(...)`. The machine needs the customer first and must belong to that customer (when both ids are known). A quote with no customer or no machine can be priced and rejected, but **not accepted**. A printed quick quote says `Customer: to be added`.
- **IDs:** a branch, a customer, a piece of equipment and a quote each have an optional UUID (`id()`); a machine also remembers its owner (`customer_id()`). A new one has **no** id; the database will give it one, and an object loaded from the database gets it with `with_id(...)`. The id is internal: it is never printed on a quote (the quote number is the name people use) A quote line (`LineItem`) has two optional ids: its own (`id()`, attached with `with_id(...)`) and the id of its quote (`quote_id()`). A new line has neither. Only the `Quote` sets a line's `quote_id`: when the quote has an id, every line already on it, and every line added after, gets that id. Nothing outside can set it, so a line can never point at the wrong quote. `quote.items()` lists the lines in the order they were added.
- **Validation:** bad lines (zero quantity, negative price) are refused with a clear `QuoteError`

```
===== Quote Q-2026-00000001: Saskatchewan =====
Date: 2026-11-17 10:58:47
Valid until: 2026-12-17
Promise date: 2026-11-20
Status: accepted
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
cargo run      # only starts the logger for now (Quoto is a library; the screens come later)
cargo run --example nhtsa_lookup -- <VIN>   # try the real NHTSA lookup (needs the internet)
cargo run --example db_check   # check the database connection (see "The database")
cargo test     # run all the unit and scenario tests
RUST_LOG=debug cargo run   # same, with a detailed log on stderr
```

## Looking up equipment
Type a serial number and a **lookup** fills in the make, model, year and kind of machine. The service writer then adds the plate and the meter reading, and `Equipment::from_draft` checks everything.
- `EquipmentLookup` is the promise: `lookup(serial)` gives `Ok(Some(draft))` (found), `Ok(None)` (not known) or `Err` (the service is down). In every case but "found", the details are typed in by hand.
- **`NhtsaLookup`** asks the US government's free [vPIC](https://vpic.nhtsa.dot.gov/api/) service. It knows road vehicles with a 17-character VIN and only trusts a clean decode. Only the VIN leaves your computer.
- **`InMemoryEquipmentLookup`** is a small catalog of made-up machines for the tests and the lookup example (it knows a tractor, which NHTSA does not).
- **`ChainedLookup`** asks several lookups in order, so road vehicles and heavy equipment can each have their own. Equipment remembers its `EquipmentSource` (entered by hand, or looked up with whom).
- Tests never touch the internet. To try the real service by hand: `cargo run --example nhtsa_lookup -- 1HGCM82633A004352`

## The database (PostgreSQL in Docker)
Quoto will keep quotes, customers and equipment in PostgreSQL (Phase 4). The database has its first tables (the provinces with their tax rates, and the branches); nothing else uses the database yet.

```bash
cp .env.example .env            # .env is ignored by git
# edit .env: put a real password in POSTGRES_PASSWORD (make one: openssl rand -base64 24)
docker compose up -d            # start PostgreSQL (waits until it is healthy)
cargo run --example db_check    # "Connected to the database at localhost:5432/quoto_dev."
cargo run --example db_setup    # build / update the tables and add the 13 provinces (safe to run again)
cargo test -- --ignored         # also run the tests that need the database
docker compose down             # stop it; the data stays in a Docker volume
```

- The database is reachable **only from your own computer** (`127.0.0.1`), and the compose file refuses to start without a password.
- Quoto reads `POSTGRES_USER`, `POSTGRES_PASSWORD`, `POSTGRES_DB` (required) and `POSTGRES_HOST`, `POSTGRES_PORT`, `POSTGRES_SSLMODE` (optional) from the environment. The names are the same ones the Postgres image uses, so compose and Quoto share one `.env`.
- **No SQL files:** the tables are built from Rust structs (SeaORM, "entity first"): `database::sync` creates a missing table and adds a missing column when the program starts. It **never drops** anything (a field removed from a struct leaves its column behind) and writes no migration file, so it is for learning and development. **Before real customer data it must be replaced by reviewable migrations.**
- **The rules live in Rust, not in the database:** `TaxRate` (above 0% and at most 100%) and `TaxRates` (a GST with an optional PST, **or** an HST alone; never both, never neither) refuse a wrong combination, when it is built and when a row is read. The table itself has no such rules, so only Quoto should write to it. There is no delete function: a province is never deleted.
- **Provinces:** one table, `provinces`, holds each province's GST, PST or HST (rates in thousandths of a percent: 5% = `5000`, 9.975% = `9975`). `seed_provinces` adds the 13 starting rows (CRA table, checked 2026-11-17) and never puts back a rate that someone changed. When the government changes a rate, `change_rates(...)` is one update (a note is required and the new date can't be before the current one); there is no history table, because rates change rarely and a quote keeps a copy of the rates it used.
- **Branches:** the `branches` table has create, read (by id, by number, or a list) and update, and **no delete**: a branch is never deleted, `set_branch_status` makes it `Inactive` (or `Active` again). A new branch is `Active`. Its number must be unique. An inactive branch still loads, but cannot start a new quote.
- **`Status`** (`active` / `inactive`) is the one "is it in use?" status for records that are never deleted (a branch now; customers, machines and users later). It is not a workflow: a quote's steps (pending, accepted...) and a repair order's steps keep their own enums.
- **Only Saskatchewan and Alberta can be loaded from a row for now**, because `Province` has only those two values; the enum grows to all 13 later.
- **If your dev database was built by the old `.sql` files**, reset it once: `docker compose down -v && docker compose up -d && cargo run --example db_setup` (this deletes the dev data).
- The database tests change data only inside a transaction that is rolled back.
- The password lives in a `Secret`, which prints as `[hidden]`; the connection is the only place that opens it.

## Keeping secrets secret
A secret is a password, an API key, a token, or a full connection address that contains one.

| Where | What goes there |
|---|---|
| **`.env` on your computer** | Local development values. Git ignores it. Never commit it. |
| **`.env.example`** | Names only, no real values. Safe to commit. |
| **Codespaces secrets** (GitHub → Settings → Codespaces → Secrets) | Secrets for your Codespace. They show up as environment variables. |
| **GitHub Actions secrets** (repo → Settings → Secrets and variables → Actions) | Secrets CI needs. CI needs none today (it uses no database and no keys). |
| **A secret manager** (your cloud's, in Phase 10) | Production secrets, injected as environment variables. |

Rules: no secret in code, in a test, in a log, in an error message, or in a screenshot. If a secret was ever committed, it is not secret any more (git keeps history), so **change it**. Turn on GitHub's secret scanning and push protection for the repo (Settings → Code security). Use a different password for each place (development, production). Tests and demos use made-up values only.

## Logging and errors
- **Logging** uses [`tracing`](https://docs.rs/tracing). `quoto::logging::init()` starts it once in `main`. Logs go to **stderr**, the level comes from `RUST_LOG` (default `info`), and logs never contain customer details.
- **Errors** are one enum, `QuoteError` (built with `thiserror`). Each error has a readable message, a stable `code()` (like `INVALID_PHONE`), a `kind()` (`InvalidInput`, `BusinessRule`, `ServiceUnavailable` or `Configuration`) and an `exit_code()`. `QuoteResult<T>` is the short name for `Result<T, QuoteError>`.
- The programs in `examples/` are the places where errors stop a run: they log the code, print the message, and exit with code 2 (wrong input), 3 (business rule), 4 (an outside service is down) or 5 (the program is set up wrongly, e.g. a missing setting).

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
├── main.rs              starts the logger and nothing else (Quoto is a library for now)
├── lib.rs               the library's front door
├── branch.rs            Branch + branch/ (location, table: the branches table and its create/read/update) (a dealership location; its province decides the taxes)
├── country.rs           Country (follows from the province)
├── errors.rs  logging.rs  money.rs  samples.rs  secret.rs
├── status.rs            Status (active / inactive) for every record that is never deleted
├── province.rs          Province + province/ (tax: the sum and the rate rules; table: the provinces table)
├── customer.rs          Customer + customer/ (address, postal_code, contact, phone_number, email)
├── database.rs          connect + sync + database/ (settings: DatabaseSettings, read from the environment)
├── equipment.rs         Equipment + equipment/ (meter, meter_type, equipment_type, identification,
│                        equipment_draft, equipment_source)
├── equipment_lookup.rs  EquipmentLookup + equipment_lookup/ (in_memory, chain, nhtsa)
└── quote.rs             Quote + quote/ (line_item, line_kind, quote_number, quote_status, number_machine)
examples/
├── nhtsa_lookup.rs      try the lookups by hand (needs the internet)
├── db_check.rs          check the database connection (needs docker compose up)
└── db_setup.rs          build / update the database tables and add the provinces (needs docker compose up)
tests/                   one file per feature, plus quote_scenarios.rs
docker-compose.yml       PostgreSQL for development
.env.example             the settings Quoto needs (copy to .env)
```
One type per file. Each feature is a file named after it (`customer.rs` holds `Customer` and lists its pieces); a feature with pieces also has a folder of the same name (`customer/`). All tests live in `tests/` (one file per feature) and only use the public API.

## Roadmap
1. ✅ Toy calculator
2. ✅ Real shapes: structs, enums, `Vec`, `Option`
3. ✅ Safe money and errors: cents, `Result`, custom errors, tests
4. 🚧 Save quotes in a database (PostgreSQL set-up ✅; provinces + tax rates table, built from a Rust struct ✅; saving quotes ⬜)
5. ⬜ A web API
6. ⬜ Quote → Repair Order
7. ⬜ Screens for service writers
8. ⬜ Invoices and PDFs
9. ⬜ Users and roles
10. ⬜ Ship it
