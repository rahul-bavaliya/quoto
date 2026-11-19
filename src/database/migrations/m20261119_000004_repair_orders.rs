// The repair order tables: `repair_order_numbers` (the counter) and `repair_orders`.
//
// A repair order points at the accepted quote it came from (`quote_id`, unique: one quote makes at
// most one repair order) and at that quote's branch, customer and machine (copied ids so lists can
// be narrowed by them). Nothing is deleted while something points at it, so every link is a
// foreign key with RESTRICT. The lines, totals and parties are read from the quote, which is final
// once accepted (see `repair_order/table.rs`).

use super::columns::{create, id, index_on, maybe_text, points_at, stamp, text};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // One row per year: the last counter given out (see `next_repair_order_number`).
        manager
            .create_table(
                create("repair_order_numbers")
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
                create("repair_orders")
                    .col(id())
                    .col(
                        ColumnDef::new(Alias::new("number"))
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Alias::new("quote_id"))
                            .uuid()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Alias::new("branch_id")).uuid().not_null())
                    .col(ColumnDef::new(Alias::new("customer_id")).uuid().not_null())
                    .col(ColumnDef::new(Alias::new("equipment_id")).uuid().not_null())
                    // The shop's own date and time, as handed in: no time zone.
                    .col(
                        ColumnDef::new(Alias::new("created_at"))
                            .timestamp()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Alias::new("due_date")).date())
                    .col(text("status"))
                    .col(maybe_text("cancellation_reason"))
                    .col(
                        ColumnDef::new(Alias::new("meter_reading"))
                            .double()
                            .not_null(),
                    )
                    .col(text("meter_type"))
                    .col(ColumnDef::new(Alias::new("started_at")).timestamp())
                    .col(ColumnDef::new(Alias::new("closed_at")).timestamp())
                    .col(stamp("updated_at"))
                    .col(
                        ColumnDef::new(Alias::new("version"))
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .foreign_key(&mut points_at("repair_orders", "quote_id", "quotes"))
                    .foreign_key(&mut points_at("repair_orders", "branch_id", "branches"))
                    .foreign_key(&mut points_at("repair_orders", "customer_id", "customers"))
                    .foreign_key(&mut points_at("repair_orders", "equipment_id", "equipment"))
                    .take(),
            )
            .await?;
        for column in ["branch_id", "customer_id", "equipment_id", "status"] {
            manager
                .create_index(index_on("repair_orders", column))
                .await?;
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Migration(String::from(
            "migrations only go forward: this one is never undone",
        )))
    }
}
