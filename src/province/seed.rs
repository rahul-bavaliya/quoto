// src/province/seed.rs: the starting rows of the `provinces` table

use super::entity::{ActiveModel, Column, Entity};
use super::store::database_error;
use super::tax_rates::TaxRates;
use crate::errors::QuoteResult;
use chrono::NaiveDate;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait};
use sea_orm::{QueryFilter, Set};

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
