// The quote tables: `quote_numbers` (the counter), `quotes` and `quote_lines`.
//
// Nothing is deleted while something points at it (a rule since Day 32), so the links are
// foreign keys with RESTRICT: the database refuses to delete a quote that has lines, or a branch,
// customer or machine a quote points at. A quote also keeps a JSON copy of the branch, customer
// and machine as they were that day (see `quote/table.rs`).

use super::columns::{create, id, index_on, maybe_text, points_at, stamp, text};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // One row per year: the last counter given out (see `next_quote_number`).
        manager
            .create_table(
                create("quote_numbers")
                    .col(
                        ColumnDef::new(Alias::new("year"))
                            .small_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("last_number"))
                            .integer()
                            .not_null(),
                    )
                    .take(),
            )
            .await?;

        manager
            .create_table(
                create("quotes")
                    .col(id())
                    .col(
                        ColumnDef::new(Alias::new("number"))
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    // The shop's own date and time, as handed in: no time zone.
                    .col(
                        ColumnDef::new(Alias::new("created_at"))
                            .timestamp()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Alias::new("expires_on")).date().not_null())
                    .col(ColumnDef::new(Alias::new("promise_date")).date())
                    .col(text("status"))
                    .col(maybe_text("rejection_reason"))
                    .col(ColumnDef::new(Alias::new("branch_id")).uuid().not_null())
                    .col(text("province_code"))
                    .col(ColumnDef::new(Alias::new("customer_id")).uuid())
                    .col(ColumnDef::new(Alias::new("equipment_id")).uuid())
                    .col(
                        ColumnDef::new(Alias::new("branch_copy"))
                            .json_binary()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Alias::new("customer_copy")).json_binary())
                    .col(ColumnDef::new(Alias::new("equipment_copy")).json_binary())
                    .col(stamp("updated_at"))
                    .foreign_key(&mut points_at("quotes", "branch_id", "branches"))
                    .foreign_key(&mut points_at("quotes", "customer_id", "customers"))
                    .foreign_key(&mut points_at("quotes", "equipment_id", "equipment"))
                    .take(),
            )
            .await?;
        for column in ["branch_id", "customer_id", "equipment_id"] {
            manager.create_index(index_on("quotes", column)).await?;
        }

        manager
            .create_table(
                create("quote_lines")
                    .col(id())
                    .col(ColumnDef::new(Alias::new("quote_id")).uuid().not_null())
                    // 0 for the first line added: a table has no order of its own.
                    .col(ColumnDef::new(Alias::new("position")).integer().not_null())
                    .col(text("kind"))
                    .col(text("description"))
                    .col(ColumnDef::new(Alias::new("quantity")).double().not_null())
                    .col(
                        ColumnDef::new(Alias::new("unit_price_cents"))
                            .big_integer()
                            .not_null(),
                    )
                    .col(stamp("created_at"))
                    .foreign_key(&mut points_at("quote_lines", "quote_id", "quotes"))
                    .take(),
            )
            .await?;
        // A quote never has two lines at one position.
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("uq-quote_lines-quote_id-position")
                    .table(Alias::new("quote_lines"))
                    .col(Alias::new("quote_id"))
                    .col(Alias::new("position"))
                    .unique()
                    .take(),
            )
            .await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Migration(String::from(
            "migrations only go forward: this one is never undone",
        )))
    }
}
