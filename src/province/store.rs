// src/province/store.rs: reading and changing the tax rates in the `provinces` table
//
// There is no delete here, on purpose: a province is never deleted, so nothing can ask for it.

use super::entity::{self, Column, Entity};
use super::tax_rates::TaxRates;
use crate::errors::{QuoteError, QuoteResult};
use chrono::NaiveDate;
use sea_orm::sea_query::Expr;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use tracing::debug;

/// A failed read or write: the cause goes to the debug log, the person sees a short message.
pub(super) fn database_error(cause: sea_orm::DbErr) -> QuoteError {
    debug!(%cause, "province query failed");
    QuoteError::DatabaseUnavailable
}

fn rates_of(row: &entity::Model) -> QuoteResult<TaxRates> {
    TaxRates::from_columns(
        row.gst_rate_thousandths,
        row.pst_rate_thousandths,
        row.hst_rate_thousandths,
    )
}

async fn row_for(db: &impl ConnectionTrait, code: &str) -> QuoteResult<entity::Model> {
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
