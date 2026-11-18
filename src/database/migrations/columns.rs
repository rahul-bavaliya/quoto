// Column shapes the migrations after the first one share, so a table's file says what is special
// about it. (The baseline migration has its own copies: a merged migration is never edited.)

use sea_orm_migration::prelude::*;

/// A required text column.
pub(super) fn text(name: &str) -> ColumnDef {
    ColumnDef::new(Alias::new(name)).text().not_null().take()
}

/// A text column that may be empty.
pub(super) fn maybe_text(name: &str) -> ColumnDef {
    ColumnDef::new(Alias::new(name)).text().take()
}

/// The id: made by the database, never by Quoto.
pub(super) fn id() -> ColumnDef {
    ColumnDef::new(Alias::new("id"))
        .uuid()
        .not_null()
        .primary_key()
        .default(Expr::cust("gen_random_uuid()"))
        .take()
}

/// A timestamp with time zone, set by the database clock when the row is made.
pub(super) fn stamp(name: &str) -> ColumnDef {
    ColumnDef::new(Alias::new(name))
        .timestamp_with_time_zone()
        .not_null()
        .default(Expr::current_timestamp())
        .take()
}

pub(super) fn create(name: &str) -> TableCreateStatement {
    Table::create()
        .table(Alias::new(name))
        .if_not_exists()
        .take()
}

/// A plain index on one column.
pub(super) fn index_on(table: &str, column: &str) -> IndexCreateStatement {
    Index::create()
        .if_not_exists()
        .name(format!("idx-{table}-{column}"))
        .table(Alias::new(table))
        .col(Alias::new(column))
        .take()
}

/// "This column must point at a row that exists, and that row can't be deleted while it does."
pub(super) fn points_at(table: &str, column: &str, other: &str) -> ForeignKeyCreateStatement {
    ForeignKey::create()
        .name(format!("fk-{table}-{column}"))
        .from(Alias::new(table), Alias::new(column))
        .to(Alias::new(other), Alias::new("id"))
        .on_delete(ForeignKeyAction::Restrict)
        .on_update(ForeignKeyAction::Restrict)
        .take()
}
