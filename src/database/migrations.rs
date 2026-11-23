// src/database/migrations.rs: the list of database changes, in the order they were made
//
// A migration is one numbered step ("step 2: add this column"). The database remembers which
// steps it has already taken (in a table called `seaql_migrations`), so each runs exactly once,
// in order, on every database: your laptop, a test database, production.
//
// Rules (see the README):
// - Never edit a migration that has been merged: add a new one after it.
// - Name them `m<date>_<number>_<what>.rs` and add them to the END of the list below.
// - They go forward only (no `down`): a step that drops or rewrites data must be written by a
//   person, reviewed in a pull request, and never undone by a button.
// - Each one is written with SeaORM's table builder (Rust, no hand-written SQL), so the pull
//   request shows exactly what will happen to the database.

use sea_orm_migration::prelude::*;

mod m20261118_000001_baseline;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20261118_000001_baseline::Migration)]
    }
}
