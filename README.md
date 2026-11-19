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
REST + JSON, built on [`axum`](https://docs.rs/axum). Today it has the **branch**, **customer**, **equipment** and **quote** calls.

```bash
docker compose up -d && cargo run --example db_setup   # the database and its tables (once; run it again after pulling a new migration)
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

**Quotes** are at `/quotes` and below. A quote is never erased: there is no DELETE, no PUT and no status call. Its status moves only through `accept`, `reject` and `reopen`, so a decision can't happen by accident.

| Call | What it does |
|---|---|
| `POST /quotes` | create: `{"branch_id": "<id>", "customer_id": null, "equipment_id": null, "items": [...]}`. Only the branch is needed (a **quick quote**); the quote is always `pending` and gets the next number of the year (`Q-2026-00000001`). The date is the shop's clock; a caller can't choose a number, id, status or date |
| `GET /quotes?customer_id=&branch_id=&equipment_id=&limit=&offset=` | newest first, with their lines; same paging as customers |
| `GET /quotes/{id}` | one quote with its branch, customer and machine **as they were when added**, its lines, `expired` (worked out today, never stored) and its totals in cents (GST and PST by the branch's province) |
| `PATCH /quotes/{id}` | change `expires_on` and/or `promise_date` (`YYYY-MM-DD`, never before the quote's own date). Both are checked first: if one is wrong, nothing changes. `null` is refused (a date can't be cleared) |
| `POST /quotes/{id}/lines` | add one line `{"description","quantity","unit_price_cents","kind"}`; `kind` is `labour`, `part`, `fee` or `credit` (a credit is sent as a **positive** price and comes off the total; it can't be bigger than the subtotal). Lines are only ever **added**: a mistake is fixed with a credit line |
| `PATCH /quotes/{id}/customer` / `.../equipment` | `{"customer_id": "<id>"}` / `{"equipment_id": "<id>"}`: add the customer (once), then the machine (once; it must be that customer's). Exactly one key, never `null` |
| `GET /quotes/{id}/pdf` | the quote as a **PDF file** (Letter size, as many pages as it needs, the column heads repeated on each page, the totals together on the last). Same facts as `GET /quotes/{id}`. The answer is sent as an **attachment** (`<quote number>.pdf`), so the docs page shows a **Download file** link (before Day 59 it printed the raw bytes as text). A browser address bar can't send the key, so use the docs page ("Try it out", then "Download file") or `curl -H "Authorization: Bearer $KEY" -o quote.pdf http://127.0.0.1:3000/quotes/<id>/pdf`. Never cached (it holds the customer's details). A rejection reason is not printed |
| `POST /quotes/{id}/accept` | needs a customer and a machine and prices that have not expired (move `expires_on` first). Final |
| `POST /quotes/{id}/reject` | `{"reason": "..."}` (not blank, at most 500 characters; the customer's own words, never logged) |
| `POST /quotes/{id}/reopen` | a rejected quote goes back to `pending`; the old reason is dropped |

**Repair orders (Day 60)** are the work a customer said yes to. A repair order is made **only from an accepted quote**, and a quote has **at most one**. It does not copy the quote: the lines, prices, taxes, customer and machine are *read from the quote*, so the two can never disagree. It adds only what the quote does not know: its own number (`RO-2026-00000001`, counted per year like quotes), a status, a due date, when it started and ended, and the meter reading when the machine came in. A repair order is never erased and has no PUT, no DELETE and no status call: it moves only through `start`, `complete` and `cancel`.

| Call | What it does |
|---|---|
| `POST /quotes/{id}/repair-order` | open one from an accepted quote. Body `{"meter_reading": 1250.5}` (or `{}`). The reading is today's meter (zero or more); leave it out to use the reading on the quote's machine, and with neither the answer is `400 METER_READING_REQUIRED`. The due date starts as the quote's promise date. `409 QUOTE_NOT_ACCEPTED` for a pending or rejected quote, `409 REPAIR_ORDER_EXISTS` if it already has one |
| `GET /repair-orders?customer_id=&branch_id=&equipment_id=&status=&limit=&offset=` | newest first, each with the quote's lines and totals. `status` is `open`, `in_progress`, `completed` or `cancelled` |
| `GET /repair-orders/{id}` | one repair order (its `version` is also the `ETag`) |
| `PATCH /repair-orders/{id}` | `{"due_date": "2026-12-01"}`, or `null` to clear it. Not before the day it was opened; not once it is completed or cancelled |
| `POST /repair-orders/{id}/start` | `open` to `in_progress` (`409 REPAIR_ORDER_NOT_OPEN` otherwise) |
| `POST /repair-orders/{id}/complete` | `in_progress` to `completed` (`409 REPAIR_ORDER_NOT_IN_PROGRESS` otherwise) |
| `POST /repair-orders/{id}/cancel` | `{"reason": "..."}` (not blank, at most 500 characters, never logged). From `open` or `in_progress`; a completed or cancelled order is final (`409 REPAIR_ORDER_CLOSED`) |

Like a quote, a repair order has a `version`, an `ETag` and an optional `If-Match` (`409 REPAIR_ORDER_CHANGED_MEANWHILE`), and opening one locks the quote's row, so eight people pressing "open" together make exactly one repair order. Not built yet (later days): the jobs inside an order (complaint, cause, correction), the technician time clock, a printable repair order, and who is allowed to do what (login and roles, Phase 9).

**The PDF (Day 54)** is written by `Quote::to_pdf(today)` in `src/quote/pdf.rs`, with no PDF library: a quote is only text and a few lines, so the file is built directly from the two standard fonts every reader has (Helvetica and Helvetica Bold; nothing is embedded, a one-page quote is about 4 KB). The same quote on the same day gives the same bytes. The standard fonts hold **Latin-1** only (English and French letters); any other character is shown as `?`. Look at one with `cargo run --example quote_pdf` (writes `quote.pdf`, made-up people). **Day 59:** a quote that can still be accepted (pending, not past its expiry date) ends with a **Customer acceptance** block: a sentence saying that signing accepts the quote at the prices above (with the date they are good until), a signature line, a date line and a printed-name line. The block is never split over two pages and never runs into the footer. An accepted, rejected or expired quote has nothing left to sign, so it has no block.

Limits (Day 53): a line has a quantity above 0 and at most 1,000,000, a unit price of at most 1,000,000.00 (so a total can never overflow), a description of 1 to 200 characters (spaces around it are dropped), and a quote holds at most 200 lines. **Two people at once (Day 55):** every quote has a `version` (0 when made, +1 with every saved change), shown in the body and as the `ETag` header. A call that changes a quote may send what it read as `If-Match: "3"` (or `3`; `*` or no header means "any"). If the quote has changed since, nothing is done and the answer is `409 QUOTE_CHANGED_MEANWHILE`: read it again and retry. Without the header the change is still checked at the moment it is saved, so two people pressing Accept and Reject together can never both win. A header that is not a version answers `400 INVALID_IF_MATCH`.

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
- Tests: `tests/api/` (no database) and `tests/database_api/` (`--ignored`; they remove their own `API-T-` branches, customers, machines and quotes (and give back the quote numbers they used) because nothing can be erased through the API). Run one with `cargo test --test api` or `cargo test --test database_api -- --ignored`.

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
cargo run --example db_setup    # run the new migrations and add the 13 provinces (safe to run again)
cargo test -- --ignored         # also run the tests that need the database
docker compose down             # stop it; the data stays in a Docker volume
```

- The database is reachable **only from your own computer** (`127.0.0.1`), and the compose file refuses to start without a password.

### Run the server in Docker with hot reload (development only)

```bash
./dev start      # database + server; the server restarts by itself when you save a file in src/
./dev logs       # watch it (Ctrl-C leaves it running)
./dev restart    # restart the server by hand
./dev stop       # stop the server (the database keeps running)
./dev status     # what is running
./dev down       # stop everything; the data stays in the Docker volume
```

- The first start is slow: it downloads Rust, installs `cargo-watch` and compiles. The tool and the build files are kept in Docker volumes, so later starts are quick.
- Each start runs `db_setup` once (it only runs new migrations and never drops anything), so a new table is created for you. A hot reload does not run it again: after a change that adds a table, run `./dev restart`.
- The server stays on `127.0.0.1:3000` (the container uses host networking, because the server refuses any other address). `.env` is read when the container starts; no secret is written in the compose file or in an image.
- Changing `.env` (for example a new `API_KEY`) needs `./dev restart`; hot reload watches only `src/`, `Cargo.toml` and `Cargo.lock`.
- This is for development: the container has the compiler and the source and runs as root. Do not use it for production.
- Quoto reads `POSTGRES_USER`, `POSTGRES_PASSWORD`, `POSTGRES_DB` (required) and `POSTGRES_HOST`, `POSTGRES_PORT`, `POSTGRES_SSLMODE` (optional) from the environment. The names are the same ones the Postgres image uses, so compose and Quoto share one `.env`.
- **Migrations:** the tables are changed only by **migrations**: numbered steps in `src/database/migrations/` (written in Rust with SeaORM's table builder, no hand-written SQL). `cargo run --example db_setup` runs the ones the database has not had yet (the database remembers them in a table called `seaql_migrations`) and adds any missing province; running it again does nothing. Rules: **never edit a migration that is merged, add a new one after it**; add it to the END of the list in `migrations.rs`; migrations only go forward (no `down`), so a step that drops or rewrites data is written by a person and reviewed in the pull request. The Rust structs say how the code *reads* a table, so a change to a table is two things: a migration and the struct. A test (`tests/database_migrations.rs`, run by hand) builds one empty database from the migrations and one from the structs and fails if they differ. `database::sync` is kept only as that reference; nothing else uses it. The first migration (`m20261118_000001_baseline`) recreates today's four tables with `IF NOT EXISTS`, so a database built earlier by `sync` adopts it with no change and keeps its rows. The migration user must be allowed to create and change tables.
- **Quote tables (Day 52):** a quote is saved in three tables (migration `m20261118_000002_quotes`): `quotes` (the quote), `quote_lines` (its lines, in order: unique `(quote_id, position)`; lines are only ever added) and `quote_numbers` (one counter row per year). A quote **points at** its branch, customer and machine (foreign keys that refuse deleting what a quote uses) **and keeps a copy** of each as it was that day (one JSON column each), so renaming a customer later never changes an old quote; reading rebuilds the parties from the copies through the normal validators and refuses a row that is damaged (link and copy disagree, line gap, bad status or number). The number `Q-<year>-<8 digits>` is handed out when the quote is saved, from the year's counter row in the same transaction: no gaps, no repeats, and a refused quote gives its number back. Calls: `create_quote`, `get_quote`, `list_quotes` (newest first; filter by customer, branch, machine), `save_quote` (decision, dates, new lines, a customer or machine added later; if someone else changed the quote meanwhile it answers `QUOTE_CHANGED_MEANWHILE`; since Day 55 this is decided by a `version` number on the quote, checked while the row is locked). Tests: `tests/database_quotes.rs` (`--ignored`; dated 2090 to 2092, so a real year's counter is never touched).
- **The rules live in Rust, not in the database:** `TaxRate` (above 0% and at most 100%) and `TaxRates` (a GST with an optional PST, **or** an HST alone; never both, never neither) refuse a wrong combination, when it is built and when a row is read. The table itself has no such rules, so only Quoto should write to it. There is no delete function: a province is never deleted.
- **Provinces:** one table, `provinces`, holds each province's GST, PST or HST (rates in thousandths of a percent: 5% = `5000`, 9.975% = `9975`). `seed_provinces` adds the 13 starting rows (CRA table, checked 2026-11-17) and never puts back a rate that someone changed. When the government changes a rate, `change_rates(...)` is one update (a note is required and the new date can't be before the current one); there is no history table, because rates change rarely and a quote keeps a copy of the rates it used.
- **Branches:** the `branches` table has create, read (by id, by number, or a list) and update, and **no delete**: a branch is never deleted, `set_branch_status` makes it `Inactive` (or `Active` again). A new branch is `Active`. Its number must be unique. An inactive branch still loads, but cannot start a new quote.
- **`Status`** (`active` / `inactive`) is the one "is it in use?" status for records that are never deleted (a branch now; customers, machines and users later). It is not a workflow: a quote's steps (pending, accepted...) and a repair order's steps keep their own enums.
- **Only Saskatchewan and Alberta can be loaded from a row for now**, because `Province` has only those two values; the enum grows to all 13 later.
- **If your dev database was built by the very old `.sql` files** (before Day 36), reset it once: `docker compose down -v && docker compose up -d && cargo run --example db_setup` (this deletes the dev data).
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
├── api.rs               the web API + api/ (settings, auth, error, log, request, branches, customers, equipment, quotes, repair_orders, docs)
├── branch.rs            Branch + branch/ (location, table: the branches table and its create/read/update) (a dealership location; its province decides the taxes)
├── country.rs           Country (follows from the province)
├── errors.rs  logging.rs  money.rs  samples.rs  secret.rs
├── status.rs            Status (active / inactive) for every record that is never deleted
├── province.rs          Province + province/ (tax: the sum and the rate rules; table: the provinces table)
├── customer.rs          Customer + customer/ (address, postal_code, contact, phone_number, email, table: the customers table and its create/read/list/update/status)
├── database.rs          connect + migrate + sync (the test reference) + database/ (settings: DatabaseSettings; migrations: the numbered steps)
├── equipment.rs         Equipment + equipment/ (meter, meter_type, equipment_type, identification,
│                        equipment_draft, equipment_source, table: the equipment table and its calls)
├── equipment_lookup.rs  EquipmentLookup + equipment_lookup/ (in_memory, chain, nhtsa)
├── quote.rs             Quote + quote/ (line_item, line_kind, quote_number, quote_status, number_machine,
│                        totals: the math, print: the text, table: the quote tables and their calls)
└── repair_order.rs      RepairOrder + repair_order/ (number, status, table: the repair order tables and their calls)
examples/
├── nhtsa_lookup.rs      try the lookups by hand (needs the internet)
├── db_check.rs          check the database connection (needs docker compose up)
└── db_setup.rs          run the migrations and add the provinces (needs docker compose up)
tests/                   one file per feature, plus quote_scenarios.rs; the two big ones are folders:
├── api/                 the API without a database (access, branches, customers, equipment, quotes, repair_orders, patch, settings, docs, support)
└── database_api/        the API with a database, run by hand (branches, customers, equipment, quotes, repair_orders, patch, logging, support)
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
