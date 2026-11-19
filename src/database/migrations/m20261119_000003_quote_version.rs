// A version number on every quote, so two people changing the same quote at the same moment
// can't silently overwrite each other (see `save_quote`): it starts at 0 and goes up by one with
// every saved change. Quotes that already exist start at 0 too.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("quotes"))
                    .add_column(
                        ColumnDef::new(Alias::new("version"))
                            .integer()
                            .not_null()
                            .default(0),
                    )
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
