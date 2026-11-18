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
cargo run      # start the web API (needs the database and API_KEY in .env; see "The web API")
cargo run --example nhtsa_lookup -- <VIN>   # try the real NHTSA lookup (needs the internet)
cargo run --example db_check   # check the database connection (see "The database")
cargo test     # run all the unit and scenario tests
RUST_LOG=debug cargo run   # same, with a detailed log on stderr
```

## The web API
REST + JSON, built on [`axum`](https://docs.rs/axum). Today it has the **branch**, **customer** and **equipment** calls.

```bash
docker compose up -d && cargo run --example db_setup   # the database and its tables (once)
# put a key in .env:  API_KEY=<output of: openssl rand -base64 32>
cargo run                                              # "the API is listening address=127.0.0.1:3000"
```
```bash
KEY=...   # the same value as API_KEY
curl -X POST http://127.0.0.1:3000/branches -H "Authorization: Bearer $KEY" -H 'Content-Type: application/json' \
  -d '{"number":"001","name":"Regina Branch","address_line1":"1 Industrial Rd","city":"Regina","province":"SK","postal_code":"S4P 3Y2","phone":"306-555-0100"}'
curl "http://127.0.0.1:3000/branches?status=active" -H "Authorization: Bearer $KEY"
```

| Call | Does |
|---|---|
| `GET /branches?status=active` | list (all of them without `?status`; `active` or `inactive`) |
| `GET /branches/{id}` | read one |
| `POST /branches` | create (always `active`) |
| `PUT /branches/{id}` | replace **all** the details (every required field must be sent; the status is not touched) |
| `PATCH /branches/{id}` | change **only the fields you send**; the rest stays as it is |
| `PATCH /branches/{id}/status` | `{"status": "inactive"}` or `{"status": "active"}` |

**Customers** work the same way (`/customers`, `/customers/{id}`, `/customers/{id}/status`), with these differences:

| Call | Does |
|---|---|
| `GET /customers?status=active&limit=50&offset=0` | list, ordered by name, **one page at a time** (`limit` defaults to 50 and is held between 1 and 200; `offset` defaults to 0). A query string it doesn't understand answers `400 INVALID_QUERY` |
| `GET /customers/{id}` | read one |
| `POST /customers` | create (always `active`; two customers may share a name) |
| `PUT /customers/{id}` | replace **all** the details (what is left out is cleared; the status is not touched) |
| `PATCH /customers/{id}` | change **only the fields you send**; the rest stays as it is |
| `PATCH /customers/{id}/status` | `{"status": "inactive"}` or `{"status": "active"}` |

**Equipment** (machines) are at `/equipment`, `/equipment/{id}`, `/equipment/{id}/status` and `/equipment/{id}/owner`:

| Call | What it does |
|---|---|
| `GET /equipment?status=&customer_id=&unowned=true&serial_number=&limit=&offset=` | list, ordered by make, model, serial number; same paging as customers |
| `POST /equipment` | create (always `active`). `customer_id` is **optional**: a machine can be entered before its customer is known |
| `PUT` / `PATCH /equipment/{id}` | replace all / change only the details sent. The **owner, status and source are not details**: sending them is refused |
| `DELETE /equipment/{id}` | **archives** the machine (it is never erased): hidden from the default list, can't go on a new quote, restorable with the status call |
| `PATCH /equipment/{id}/owner` | `{"customer_id": "<id>"}` gives it to a customer (a **sale** is just this call again), `{"customer_id": null}` to nobody. The body must be exactly that one key |

One customer can own many machines. **One physical machine is one record:** the serial number is unique (any case), so entering it twice answers `409 DUPLICATE_SERIAL_NUMBER`; find the machine by `?serial_number=` and change its owner instead. A plate is not unique. Only the current owner is kept (no history yet). Machines identify real vehicles: made-up ones only until real login exists, and nothing about a machine (serial, plate, make, model, ids) is ever logged.

A customer is never deleted either. An **inactive customer can't be put on a new quote** (`CUSTOMER_INACTIVE`, 409). Customers are people's details: **use made-up people only until real login exists (Phase 9).**

**PATCH, the partial change** (the same for every record from now on): leave a field out and it is not touched; send `null` to clear an optional field (`"company": null`); `null` for a required field (name, phone, city...) is refused, and so are `id`, `status`, `country` and unknown fields. An empty body `{}` is refused as "nothing to change". The sent fields are laid over the saved record and the result goes through the same checks as a POST, so a refused PATCH changes nothing.
```bash
curl -X PATCH http://127.0.0.1:3000/customers/<id> -H "Authorization: Bearer $KEY" -H 'Content-Type: application/json' -d '{"email": null, "phone": "306-555-0199"}'
```

**Try it in the browser:** with the server running, open <http://127.0.0.1:3000/docs> (in Codespaces: the Ports tab, port 3000, "Open in Browser", then add `/docs`). It lists every call and has a "Try it out" button on each. Click **Authorize** once and paste your `API_KEY`. The same list as data is at `/openapi.json`.

There is **no DELETE** for branches and customers (they are made inactive); `DELETE /equipment/{id}` only archives. A failed call answers `{"code": "...", "message": "..."}` with the library's own code; the status is 400 (wrong input), 401 (no or wrong key), 404 (no such branch, customer or machine), 409 (a rule refused it, e.g. a number already used) or 503 (database down).

- **The docs page is the one thing that opens without a key** (`/docs` and `/openapi.json`): a browser can't send a key to load a page, and they show only the shape of the API, never a branch. Everything else, even an address that doesn't exist, needs the key. The list is built from the notes above each handler, so it can't drift from the code. Keep Codespaces port 3000 **Private**, and switch the docs off before the API is ever put on the internet (Phase 10).
- **Every call needs the key** (`Authorization: Bearer <key>`), even for a route that doesn't exist. This is a stand-in until real login and roles (Phase 9). The key is read from `API_KEY` into a `Secret`, compared in constant time, never logged and never sent back. There is no default key.
- **It only listens on `127.0.0.1`.** The key travels as plain text, so the API refuses any other address until there is HTTPS (Phase 10).
- Bodies over 16 KB are refused. A field the call doesn't know (like `status` on create) is refused.
- The JSON is only translated: every rule (phone, postal code, province, unique number) is the library's, the same as for any other caller.
- `cargo run` does not build the tables; run `cargo run --example db_setup` first (the API answers 503 until the tables exist).
- Tests: `tests/api/` (no database) and `tests/database_api/` (`--ignored`; they remove their own `API-T-` branches, customers and machines because nothing can be erased through the API). Run one with `cargo test --test api` or `cargo test --test database_api -- --ignored`.

## Looking up equipment
Type a serial number and a **lookup** fills in the make, model, year and kind of machine. The service writer then adds the plate and the meter reading, and `Equipment::from_draft` checks everything.
- `EquipmentLookup` is the promise: `lookup(serial)` gives `Ok(Some(draft))` (found), `Ok(None)` (not known) or `Err` (the service is down). In every case but "found", the details are typed in by hand.
- **`NhtsaLookup`** asks the US government's free [vPIC](https://vpic.nhtsa.dot.gov/api/) service. It knows road vehicles with a 17-character VIN and only trusts a clean decode. Only the VIN leaves your computer.
- **`InMemoryEquipmentLookup`** is a small catalog of made-up machines for the tests and the lookup example (it knows a tractor, which NHTSA does not).
- **`ChainedLookup`** asks several lookups in order, so road vehicles and heavy equipment can each have their own. Equipment remembers its `EquipmentSource` (entered by hand, or looked up with whom).
- Tests never touch the internet. To try the real service by hand: `cargo run --example nhtsa_lookup -- 1HGCM82633A004352`

## The database (PostgreSQL in Docker)
Quoto will keep quotes, customers and equipment in PostgreSQL (Phase 4). The database has its first tables (the provinces with their tax rates, the branches, the customers and the equipment); nothing else uses the database yet.

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
- **What the API logs** (rules: `claude/logging-rules.md` in the project docs): **one line per call**, like `api_request request_id=7 method=POST route="/branches" status=201 elapsed_ms=5 code="none"`. It holds the call number (also sent back in the `x-request-id` header), the method, the route *pattern*, the status, the time and the error *code*. It **never** holds the key or any header, a body, the real path or query, an id, or anything about a branch or customer; `tests/api_logging.rs` makes calls full of secret values and fails if one reaches the log. The level follows the status: 2xx `info`, 4xx `warn` (the caller's mistake), 5xx `error` (our side failed). Keep `RUST_LOG=debug` for your own computer only.
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
├── main.rs              starts the web API (and is the one place where errors stop the program)
├── lib.rs               the library's front door
├── api.rs               the web API + api/ (settings, auth, error, log, request, branches, customers, equipment, docs)
├── branch.rs            Branch + branch/ (location, table: the branches table and its create/read/update) (a dealership location; its province decides the taxes)
├── country.rs           Country (follows from the province)
├── errors.rs  logging.rs  money.rs  samples.rs  secret.rs
├── status.rs            Status (active / inactive) for every record that is never deleted
├── province.rs          Province + province/ (tax: the sum and the rate rules; table: the provinces table)
├── customer.rs          Customer + customer/ (address, postal_code, contact, phone_number, email, table: the customers table and its create/read/list/update/status)
├── database.rs          connect + sync + database/ (settings: DatabaseSettings, read from the environment)
├── equipment.rs         Equipment + equipment/ (meter, meter_type, equipment_type, identification,
│                        equipment_draft, equipment_source, table: the equipment table and its calls)
├── equipment_lookup.rs  EquipmentLookup + equipment_lookup/ (in_memory, chain, nhtsa)
└── quote.rs             Quote + quote/ (line_item, line_kind, quote_number, quote_status, number_machine,
                         totals: the math, print: the text)
examples/
├── nhtsa_lookup.rs      try the lookups by hand (needs the internet)
├── db_check.rs          check the database connection (needs docker compose up)
└── db_setup.rs          build / update the database tables and add the provinces (needs docker compose up)
tests/                   one file per feature, plus quote_scenarios.rs; the two big ones are folders:
├── api/                 the API without a database (access, branches, customers, equipment, patch, settings, docs, support)
└── database_api/        the API with a database, run by hand (branches, customers, equipment, patch, logging, support)
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
