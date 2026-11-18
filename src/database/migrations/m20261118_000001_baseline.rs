// The first migration: the four tables Quoto had before migrations existed (provinces, branches,
// customers, equipment), exactly as `database::sync` built them.
//
// It is written with SeaORM's table builder (Rust, no hand-written SQL). Every table and index
// says "if not exists", so a database that was already built by `sync` (a developer's laptop)
// adopts this step with no change, and an empty one is built from nothing. Nothing here drops or
// rewrites anything. `tests/database_migrations.rs` proves it builds the same tables the Rust
// structs describe.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// A required text column.
fn text(name: &str) -> ColumnDef {
    ColumnDef::new(Alias::new(name)).text().not_null().take()
}

/// A text column that may be empty.
fn maybe_text(name: &str) -> ColumnDef {
    ColumnDef::new(Alias::new(name)).text().take()
}

/// A required text column that no two rows may share.
fn unique_text(name: &str) -> ColumnDef {
    ColumnDef::new(Alias::new(name))
        .text()
        .not_null()
        .unique_key()
        .take()
}

/// The id: made by the database, never by Quoto.
fn id() -> ColumnDef {
    ColumnDef::new(Alias::new("id"))
        .uuid()
        .not_null()
        .primary_key()
        .default(Expr::cust("gen_random_uuid()"))
        .take()
}

/// `created_at` or `updated_at`: the database clock.
fn stamp(name: &str) -> ColumnDef {
    ColumnDef::new(Alias::new(name))
        .timestamp_with_time_zone()
        .not_null()
        .default(Expr::current_timestamp())
        .take()
}

fn create(name: &str) -> TableCreateStatement {
    Table::create()
        .table(Alias::new(name))
        .if_not_exists()
        .take()
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                create("provinces")
                    .col(id())
                    .col(unique_text("code"))
                    .col(unique_text("name"))
                    .col(text("country_code"))
                    // Thousandths of a percent; empty means "no such tax".
                    .col(ColumnDef::new(Alias::new("gst_rate_thousandths")).integer())
                    .col(ColumnDef::new(Alias::new("pst_rate_thousandths")).integer())
                    .col(ColumnDef::new(Alias::new("hst_rate_thousandths")).integer())
                    .col(
                        ColumnDef::new(Alias::new("effective_from"))
                            .date()
                            .not_null(),
                    )
                    .col(text("note"))
                    .col(stamp("created_at"))
                    .col(stamp("updated_at"))
                    .take(),
            )
            .await?;

        manager
            .create_table(
                create("branches")
                    .col(id())
                    .col(unique_text("number"))
                    .col(text("name"))
                    .col(text("address_line1"))
                    .col(maybe_text("address_line2"))
                    .col(text("city"))
                    .col(text("province_code"))
                    .col(text("country_code"))
                    .col(text("postal_code"))
                    .col(text("phone"))
                    .col(ColumnDef::new(Alias::new("latitude")).double())
                    .col(ColumnDef::new(Alias::new("longitude")).double())
                    .col(text("status"))
                    .col(stamp("created_at"))
                    .col(stamp("updated_at"))
                    .take(),
            )
            .await?;

        manager
            .create_table(
                create("customers")
                    .col(id())
                    .col(text("name"))
                    .col(maybe_text("company"))
                    .col(maybe_text("designation"))
                    .col(text("phone"))
                    .col(maybe_text("alternate_phone"))
                    .col(maybe_text("email"))
                    .col(text("address_line1"))
                    .col(maybe_text("address_line2"))
                    .col(text("city"))
                    .col(text("province_code"))
                    .col(text("country_code"))
                    .col(text("postal_code"))
                    .col(text("status"))
                    .col(stamp("created_at"))
                    .col(stamp("updated_at"))
                    .take(),
            )
            .await?;

        // No foreign key on customer_id: customers are never deleted and the owner is checked in Rust.
        manager
            .create_table(
                create("equipment")
                    .col(id())
                    .col(ColumnDef::new(Alias::new("customer_id")).uuid())
                    .col(text("make"))
                    .col(text("model"))
                    .col(ColumnDef::new(Alias::new("model_year")).big_integer())
                    // Many machines may have no serial number; two may never share one.
                    .col(
                        ColumnDef::new(Alias::new("serial_number"))
                            .text()
                            .unique_key(),
                    )
                    .col(maybe_text("plate"))
                    .col(ColumnDef::new(Alias::new("meter_reading")).double())
                    .col(text("meter_type"))
                    .col(text("equipment_type"))
                    .col(text("source"))
                    .col(text("status"))
                    .col(stamp("created_at"))
                    .col(stamp("updated_at"))
                    .take(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-equipment-customer_id")
                    .table(Alias::new("equipment"))
                    .col(Alias::new("customer_id"))
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
