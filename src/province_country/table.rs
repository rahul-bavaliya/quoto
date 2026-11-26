// src/province/table.rs: the `provinces` table in the database, its idempotent CRA seeding,
// rate lookups, and rate updates.
//
// There is no delete function: a province is never deleted. Rates are stored as integer
// thousandths of a percent (5% = 5000, 9.975% = 9975), and tax-combination rules
// (GST + optional PST, or HST alone; never both, never neither) are validated in Rust.

use crate::errors::{AppError, AppResult};
use crate::province_country::{Country, Province, TaxRateInfo};
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::DateTimeUtc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, QueryOrder, Set};
use tracing::debug;

use row::{ActiveModel, Column, Entity};

mod row {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize)]
    #[sea_orm(table_name = "provinces")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
        pub code: String,
        #[sea_orm(column_type = "Text")]
        pub name: String,
        #[sea_orm(column_type = "Text")]
        pub country_code: String,
        pub gst_rate_milli_pct: Option<i32>,
        pub pst_rate_milli_pct: Option<i32>,
        pub hst_rate_milli_pct: Option<i32>,
        #[sea_orm(column_type = "Text")]
        pub note: Option<String>,
        pub effective_date: Option<DateTimeUtc>,
        #[sea_orm(default_expr = "Expr::current_timestamp()")]
        pub created_at: DateTimeUtc,
        #[sea_orm(default_expr = "Expr::current_timestamp()")]
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

fn rates_to_columns(rates: &TaxRateInfo) -> (Option<i32>, Option<i32>, Option<i32>) {
    (
        rates.gst_milli_pct,
        rates.pst_milli_pct,
        rates.hst_milli_pct,
    )
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
        row.gst_rate_milli_pct,
        row.pst_rate_milli_pct,
        row.hst_rate_milli_pct,
    )?;

    Ok(ProvinceTaxRecord {
        code: trimmed_code,
        name: row.name,
        country,
        rates,
        note: row.note,
        effective_date: row.effective_date,
        updated_at: row.updated_at,
    })
}

struct SeedProvince {
    code: &'static str,
    name: &'static str,
    gst: Option<i32>,
    pst: Option<i32>,
    hst: Option<i32>,
}

/// All 13 Canadian provinces and territories (CRA rates in thousandths of a percent).
const CRA_PROVINCES: &[SeedProvince] = &[
    SeedProvince {
        code: "AB",
        name: "Alberta",
        gst: Some(5000),
        pst: None,
        hst: None,
    },
    SeedProvince {
        code: "BC",
        name: "British Columbia",
        gst: Some(5000),
        pst: Some(7000),
        hst: None,
    },
    SeedProvince {
        code: "MB",
        name: "Manitoba",
        gst: Some(5000),
        pst: Some(7000),
        hst: None,
    },
    SeedProvince {
        code: "NB",
        name: "New Brunswick",
        gst: None,
        pst: None,
        hst: Some(15000),
    },
    SeedProvince {
        code: "NL",
        name: "Newfoundland and Labrador",
        gst: None,
        pst: None,
        hst: Some(15000),
    },
    SeedProvince {
        code: "NS",
        name: "Nova Scotia",
        gst: None,
        pst: None,
        hst: Some(15000),
    },
    SeedProvince {
        code: "NT",
        name: "Northwest Territories",
        gst: Some(5000),
        pst: None,
        hst: None,
    },
    SeedProvince {
        code: "NU",
        name: "Nunavut",
        gst: Some(5000),
        pst: None,
        hst: None,
    },
    SeedProvince {
        code: "ON",
        name: "Ontario",
        gst: None,
        pst: None,
        hst: Some(13000),
    },
    SeedProvince {
        code: "PE",
        name: "Prince Edward Island",
        gst: None,
        pst: None,
        hst: Some(15000),
    },
    SeedProvince {
        code: "QC",
        name: "Quebec",
        gst: Some(5000),
        pst: Some(9975),
        hst: None,
    },
    SeedProvince {
        code: "SK",
        name: "Saskatchewan",
        gst: Some(5000),
        pst: Some(6000),
        hst: None,
    },
    SeedProvince {
        code: "YT",
        name: "Yukon",
        gst: Some(5000),
        pst: None,
        hst: None,
    },
];

/// Seeds the 13 provinces/territories idempotently.
/// Inserts any missing row and never overwrites rates that were intentionally updated.
pub async fn seed_provinces(db: &impl ConnectionTrait) -> AppResult<()> {
    let default_effective = DateTime::parse_from_rfc3339("2026-11-17T00:00:00Z")
        .expect("valid constant timestamp")
        .with_timezone(&Utc);

    for seed in CRA_PROVINCES {
        let existing = Entity::find_by_id(seed.code)
            .one(db)
            .await
            .map_err(database_error)?;

        if existing.is_some() {
            continue;
        }

        // Validate through domain rules before inserting.
        let rates = TaxRateInfo::new(seed.gst, seed.pst, seed.hst)?;
        let (gst, pst, hst) = rates_to_columns(&rates);

        let model = ActiveModel {
            code: Set(seed.code.to_string()),
            name: Set(seed.name.to_string()),
            country_code: Set(Country::Canada.code().to_string()),
            gst_rate_milli_pct: Set(gst),
            pst_rate_milli_pct: Set(pst),
            hst_rate_milli_pct: Set(hst),
            note: Set(Some("Initial CRA seed".to_string())),
            effective_date: Set(Some(default_effective)),
            ..Default::default()
        };

        model.insert(db).await.map_err(database_error)?;
    }

    Ok(())
}

/// Fetches a single province's tax record by its two-letter code (e.g., `"SK"`).
pub async fn get_province_tax_rates(
    db: &impl ConnectionTrait,
    code: &str,
) -> AppResult<ProvinceTaxRecord> {
    let normalized = code.trim().to_ascii_uppercase();
    let row = Entity::find_by_id(&normalized)
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
    let existing = Entity::find_by_id(code)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(|| AppError::InvalidProvince {
            code: code.to_string(),
        })?;

    if let Some(current_effective) = existing.effective_date {
        if effective_date < current_effective {
            return Err(AppError::InvalidEffectiveDate);
        }
    }

    let (gst, pst, hst) = rates_to_columns(&new_rates);
    let mut active: ActiveModel = existing.into();
    active.gst_rate_milli_pct = Set(gst);
    active.pst_rate_milli_pct = Set(pst);
    active.hst_rate_milli_pct = Set(hst);
    active.note = Set(Some(trimmed_note.to_string()));
    active.effective_date = Set(Some(effective_date));
    active.updated_at = Set(Utc::now());

    let updated = active.update(db).await.map_err(database_error)?;
    record_from_row(updated)
}
