// src/province/entity.rs: the `provinces` table, described as a Rust struct
//
// SeaORM builds the table from this struct (see `database::sync`): add a field here and the
// next sync adds the column. A removed field leaves its column behind (sync never drops).
// Only code in this crate touches the raw rows; everyone else goes through `TaxRates`,
// which checks the rules the database does not.

use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "provinces")]
pub struct Model {
    /// Made by the database, never by Quoto.
    #[sea_orm(
        primary_key,
        auto_increment = false,
        default_expr = "Expr::cust(\"gen_random_uuid()\")"
    )]
    pub id: Uuid,
    /// Two capital letters, e.g. "SK". Other tables will point at it.
    #[sea_orm(unique, column_type = "Text")]
    pub code: String,
    #[sea_orm(unique, column_type = "Text")]
    pub name: String,
    #[sea_orm(column_type = "Text")]
    pub country_code: String,
    // The three rate columns, in thousandths of a percent. Empty means "no such tax".
    // `TaxRates::from_columns` is the only thing that decides which mixes are allowed.
    pub gst_rate_thousandths: Option<i32>,
    pub pst_rate_thousandths: Option<i32>,
    pub hst_rate_thousandths: Option<i32>,
    /// The day these rates took effect.
    pub effective_from: Date,
    /// Where the numbers came from, or why they changed.
    #[sea_orm(column_type = "Text")]
    pub note: String,
    #[sea_orm(default_expr = "Expr::current_timestamp()")]
    pub created_at: DateTimeUtc,
    #[sea_orm(default_expr = "Expr::current_timestamp()")]
    pub updated_at: DateTimeUtc,
}

impl ActiveModelBehavior for ActiveModel {}
