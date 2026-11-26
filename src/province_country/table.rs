// src/province_country/table.rs: the `provinces` table in the database: rate lookups and rate updates.
//
// The 13 starting rows are put there by the migration `m20261126_000003_create_provinces`, so
// there is one list of them, not two. There is no delete function: a province is never deleted.
// Rates are stored as integer thousandths of a percent (5% = 5000, 9.975% = 9975), and the
// tax-combination rules (GST + optional PST, or HST alone; never both, never neither) are
// validated in Rust.

use crate::errors::{AppError, AppResult};
use crate::province_country::{Country, Province, TaxRateInfo};
use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::DateTimeUtc;
use sea_orm::sea_query::Expr;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use tracing::debug;

use row::{Column, Entity};

mod row {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "provinces")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        #[sea_orm(column_type = "Text", unique)]
        pub code: String,
        #[sea_orm(column_type = "Text", unique)]
        pub name: String,
        #[sea_orm(column_type = "Text")]
        pub country_code: String,
        pub gst_rate_thousandths: Option<i32>,
        pub pst_rate_thousandths: Option<i32>,
        pub hst_rate_thousandths: Option<i32>,
        pub effective_from: Date,
        #[sea_orm(column_type = "Text")]
        pub note: String,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// A province/territory row loaded from the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvinceTaxRecord {
    pub code: String,
    pub name: String,
    pub country: Country,
    pub rates: TaxRateInfo,
    pub note: Option<String>,
    pub effective_date: Option<DateTime<Utc>>,
    pub updated_at: DateTimeUtc,
}

fn database_error(cause: sea_orm::DbErr) -> AppError {
    debug!(%cause, "province query failed");
    AppError::DatabaseUnavailable
}

/// The table keeps a date; the record shows it as midnight UTC.
fn start_of_day(date: NaiveDate) -> DateTime<Utc> {
    date.and_hms_opt(0, 0, 0)
        .expect("midnight is a valid time")
        .and_utc()
}

fn record_from_row(row: row::Model) -> AppResult<ProvinceTaxRecord> {
    let trimmed_code = row.code.trim().to_ascii_uppercase();
    if trimmed_code.is_empty() || row.name.trim().is_empty() {
        return Err(AppError::InvalidProvince { code: row.code });
    }

    let country = match row.country_code.trim().to_ascii_uppercase().as_str() {
        "CA" => Country::Canada,
        _ => {
            return Err(AppError::InvalidProvince {
                code: row.country_code,
            });
        }
    };

    let rates = TaxRateInfo::new(
        row.gst_rate_thousandths,
        row.pst_rate_thousandths,
        row.hst_rate_thousandths,
    )?;

    Ok(ProvinceTaxRecord {
        code: trimmed_code,
        name: row.name,
        country,
        rates,
        note: Some(row.note),
        effective_date: Some(start_of_day(row.effective_from)),
        updated_at: row.updated_at,
    })
}

/// Fetches a single province's tax record by its two-letter code (e.g., `"SK"`).
pub async fn get_province_tax_rates(
    db: &impl ConnectionTrait,
    code: &str,
) -> AppResult<ProvinceTaxRecord> {
    let normalized = code.trim().to_ascii_uppercase();
    let row = Entity::find()
        .filter(Column::Code.eq(normalized.as_str()))
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or(AppError::InvalidProvince { code: normalized })?;

    record_from_row(row)
}

/// Lists all seeded provinces ordered by province code.
pub async fn list_province_tax_rates(
    db: &impl ConnectionTrait,
) -> AppResult<Vec<ProvinceTaxRecord>> {
    let rows = Entity::find()
        .order_by_asc(Column::Code)
        .all(db)
        .await
        .map_err(database_error)?;

    rows.into_iter().map(record_from_row).collect()
}

/// Updates a province's tax rates in one update.
/// Requires a non-blank note and an `effective_date` not earlier than the current one.
pub async fn change_rates(
    db: &impl ConnectionTrait,
    province: Province,
    new_rates: TaxRateInfo,
    note: &str,
    effective_date: DateTimeUtc,
) -> AppResult<ProvinceTaxRecord> {
    let trimmed_note = note.trim();
    if trimmed_note.is_empty() {
        return Err(AppError::MissingField { field: "note" });
    }

    let code = province.code();
    let existing = Entity::find()
        .filter(Column::Code.eq(code))
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(|| AppError::InvalidProvince {
            code: code.to_string(),
        })?;

    let new_date = effective_date.date_naive();
    if new_date < existing.effective_from {
        return Err(AppError::InvalidEffectiveDate);
    }

    Entity::update_many()
        .col_expr(
            Column::GstRateThousandths,
            Expr::value(new_rates.gst_milli_pct),
        )
        .col_expr(
            Column::PstRateThousandths,
            Expr::value(new_rates.pst_milli_pct),
        )
        .col_expr(
            Column::HstRateThousandths,
            Expr::value(new_rates.hst_milli_pct),
        )
        .col_expr(Column::Note, Expr::value(trimmed_note))
        .col_expr(Column::EffectiveFrom, Expr::value(new_date))
        .col_expr(Column::UpdatedAt, Expr::current_timestamp())
        .filter(Column::Id.eq(existing.id))
        .exec(db)
        .await
        .map_err(database_error)?;

    get_province_tax_rates(db, code).await
}
