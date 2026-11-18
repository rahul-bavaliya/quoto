// src/province/table.rs: the `provinces` table in the database
//
// The table is described by a Rust struct (`row`): `database::sync` builds it, and adding a field
// adds the column (a removed field leaves its column behind: sync never drops). The functions below
// are the only code that touches the raw rows; everyone else gets `TaxRates`, which checks the rules
// the table does not. There is no delete function on purpose: a province is never deleted.

use super::tax::TaxRates;
use crate::errors::{QuoteError, QuoteResult};
use chrono::NaiveDate;
use sea_orm::sea_query::Expr;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait};
use sea_orm::{QueryFilter, QueryOrder, Set};
use tracing::debug;

pub(crate) use row::Entity as ProvinceTable;
use row::{ActiveModel, Column, Entity};

/// The `provinces` table as a struct.
mod row {
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
}

// ===== the starting rows =====

/// The note on every starting row: where the numbers came from.
const SOURCE: &str = "CRA GST/HST rates by province, checked 2026-11-17";

/// One starting row: (code, name, GST, PST, HST).
type SeedRow = (
    &'static str,
    &'static str,
    Option<i32>,
    Option<i32>,
    Option<i32>,
);

/// The 13 rows, in thousandths of a percent (5% = 5000, 9.975% = 9975).
/// Quebec's QST and Manitoba's RST are kept as the PST. NS cut its HST to 14% on 2025-04-01.
const PROVINCES: [SeedRow; 13] = [
    ("AB", "Alberta", Some(5000), None, None),
    ("BC", "British Columbia", Some(5000), Some(7000), None),
    ("MB", "Manitoba", Some(5000), Some(7000), None),
    ("NB", "New Brunswick", None, None, Some(15_000)),
    ("NL", "Newfoundland and Labrador", None, None, Some(15_000)),
    ("NS", "Nova Scotia", None, None, Some(14_000)),
    ("NT", "Northwest Territories", Some(5000), None, None),
    ("NU", "Nunavut", Some(5000), None, None),
    ("ON", "Ontario", None, None, Some(13_000)),
    ("PE", "Prince Edward Island", None, None, Some(15_000)),
    ("QC", "Quebec", Some(5000), Some(9975), None),
    ("SK", "Saskatchewan", Some(5000), Some(6000), None),
    ("YT", "Yukon", Some(5000), None, None),
];

/// Adds the 13 provinces that are **missing**. A province that is already there is left alone
/// (so a rate someone changed is never put back), which makes running it twice safe.
/// Every row goes through `TaxRates::from_columns`, so the starting data is checked too.
pub async fn seed_provinces(db: &impl ConnectionTrait) -> QuoteResult<()> {
    // "In force since at least this date": the rates were checked on 2026-11-17.
    let in_force_since = NaiveDate::from_ymd_opt(2025, 4, 1).expect("a real date");

    for (code, name, gst, pst, hst) in PROVINCES {
        let exists = Entity::find()
            .filter(Column::Code.eq(code))
            .count(db)
            .await
            .map_err(database_error)?
            > 0;
        if exists {
            continue;
        }
        let rates = TaxRates::from_columns(gst, pst, hst)?;
        let (gst, pst, hst) = rates.to_columns();
        ActiveModel {
            code: Set(code.to_string()),
            name: Set(name.to_string()),
            country_code: Set("CA".to_string()),
            gst_rate_thousandths: Set(gst),
            pst_rate_thousandths: Set(pst),
            hst_rate_thousandths: Set(hst),
            effective_from: Set(in_force_since),
            note: Set(SOURCE.to_string()),
            ..Default::default() // the id and both timestamps are filled in by the database
        }
        .insert(db)
        .await
        .map_err(database_error)?;
    }
    Ok(())
}

// ===== reading and changing the rates =====

/// A failed read or write: the cause goes to the debug log, the person sees a short message.
pub(super) fn database_error(cause: sea_orm::DbErr) -> QuoteError {
    debug!(%cause, "province query failed");
    QuoteError::DatabaseUnavailable
}

fn rates_of(row: &row::Model) -> QuoteResult<TaxRates> {
    TaxRates::from_columns(
        row.gst_rate_thousandths,
        row.pst_rate_thousandths,
        row.hst_rate_thousandths,
    )
}

async fn row_for(db: &impl ConnectionTrait, code: &str) -> QuoteResult<row::Model> {
    Entity::find()
        .filter(Column::Code.eq(code))
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(|| QuoteError::UnknownProvince {
            code: code.to_string(),
        })
}

/// The taxes of one province, e.g. `rates_for(&db, "SK")`. A damaged row is refused here
/// (see `TaxRates::from_columns`), so a wrong combination never reaches a quote.
pub async fn rates_for(db: &impl ConnectionTrait, code: &str) -> QuoteResult<TaxRates> {
    rates_of(&row_for(db, code).await?)
}

/// Every province's code with its taxes, ordered by code.
pub async fn all_rates(db: &impl ConnectionTrait) -> QuoteResult<Vec<(String, TaxRates)>> {
    Entity::find()
        .order_by_asc(Column::Code)
        .all(db)
        .await
        .map_err(database_error)?
        .iter()
        .map(|row| Ok((row.code.clone(), rates_of(row)?)))
        .collect()
}

/// Changes a province's rates when the government changes them (one update).
///
/// The note is required (say where the new numbers come from), and the new start date can't be
/// before the date the current rates started. Nothing else about the province can be changed here.
/// The old rates are not kept: a quote keeps a copy of the rates it used.
pub async fn change_rates(
    db: &impl ConnectionTrait,
    code: &str,
    rates: &TaxRates,
    effective_from: NaiveDate,
    note: &str,
) -> QuoteResult<()> {
    let note = note.trim();
    if note.is_empty() {
        return Err(QuoteError::MissingField { field: "note" });
    }
    let current = row_for(db, code).await?.effective_from;
    if effective_from < current {
        return Err(QuoteError::RatesDateBeforeCurrent {
            date: effective_from,
            current,
        });
    }

    let (gst, pst, hst) = rates.to_columns();
    Entity::update_many()
        .col_expr(Column::GstRateThousandths, Expr::value(gst))
        .col_expr(Column::PstRateThousandths, Expr::value(pst))
        .col_expr(Column::HstRateThousandths, Expr::value(hst))
        .col_expr(Column::EffectiveFrom, Expr::value(effective_from))
        .col_expr(Column::Note, Expr::value(note))
        .col_expr(Column::UpdatedAt, Expr::current_timestamp())
        .filter(Column::Code.eq(code))
        .exec(db)
        .await
        .map_err(database_error)?;
    Ok(())
}
