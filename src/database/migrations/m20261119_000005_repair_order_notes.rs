// The three paragraphs a repair order can carry: the customer's complaint, the cause the
// technician found, and the correction that was made. All optional free text, so the columns are
// empty (NULL) for every repair order that exists already.

use super::columns::maybe_text;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("repair_orders"))
                    .add_column(maybe_text("complaint"))
                    .add_column(maybe_text("cause"))
                    .add_column(maybe_text("correction"))
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
