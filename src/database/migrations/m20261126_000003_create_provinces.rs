// src/database/migrations/m20261126_000003_create_provinces.rs: creates (if absent) and seeds
// the `provinces` table with all 13 Canadian provinces and territories.
//
// Tax-rate combination rules (GST + optional PST, or HST alone; never both, never neither)
// and rate bounds are validated in Rust rather than SQL check constraints. Rates are stored
// in thousandths of a percent (5% = 5000, 9.975% = 9975).

use chrono::NaiveDate;
use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_query::{Expr, Query};

#[derive(DeriveMigrationName)]
pub struct Migration;

struct ProvinceSeed {
    code: &'static str,
    name: &'static str,
    country_code: &'static str,
    gst_rate_thousandths: Option<i32>,
    pst_rate_thousandths: Option<i32>,
    hst_rate_thousandths: Option<i32>,
    effective_year: i32,
    effective_month: u32,
    effective_day: u32,
    note: &'static str,
}

const CANADIAN_PROVINCES: &[ProvinceSeed] = &[
    ProvinceSeed {
        code: "AB",
        name: "Alberta",
        country_code: "CA",
        gst_rate_thousandths: Some(5000),
        pst_rate_thousandths: None,
        hst_rate_thousandths: None,
        effective_year: 2008,
        effective_month: 1,
        effective_day: 1,
        note: "Initial CRA seed",
    },
    ProvinceSeed {
        code: "BC",
        name: "British Columbia",
        country_code: "CA",
        gst_rate_thousandths: Some(5000),
        pst_rate_thousandths: Some(7000),
        hst_rate_thousandths: None,
        effective_year: 2013,
        effective_month: 4,
        effective_day: 1,
        note: "Initial CRA seed",
    },
    ProvinceSeed {
        code: "MB",
        name: "Manitoba",
        country_code: "CA",
        gst_rate_thousandths: Some(5000),
        pst_rate_thousandths: Some(7000),
        hst_rate_thousandths: None,
        effective_year: 2019,
        effective_month: 7,
        effective_day: 1,
        note: "Initial CRA seed",
    },
    ProvinceSeed {
        code: "NB",
        name: "New Brunswick",
        country_code: "CA",
        gst_rate_thousandths: None,
        pst_rate_thousandths: None,
        hst_rate_thousandths: Some(15000),
        effective_year: 2016,
        effective_month: 7,
        effective_day: 1,
        note: "Initial CRA seed",
    },
    ProvinceSeed {
        code: "NL",
        name: "Newfoundland and Labrador",
        country_code: "CA",
        gst_rate_thousandths: None,
        pst_rate_thousandths: None,
        hst_rate_thousandths: Some(15000),
        effective_year: 2016,
        effective_month: 7,
        effective_day: 1,
        note: "Initial CRA seed",
    },
    ProvinceSeed {
        code: "NS",
        name: "Nova Scotia",
        country_code: "CA",
        gst_rate_thousandths: None,
        pst_rate_thousandths: None,
        hst_rate_thousandths: Some(14000),
        effective_year: 2025,
        effective_month: 4,
        effective_day: 1,
        note: "Initial CRA seed",
    },
    ProvinceSeed {
        code: "NT",
        name: "Northwest Territories",
        country_code: "CA",
        gst_rate_thousandths: Some(5000),
        pst_rate_thousandths: None,
        hst_rate_thousandths: None,
        effective_year: 2008,
        effective_month: 1,
        effective_day: 1,
        note: "Initial CRA seed",
    },
    ProvinceSeed {
        code: "NU",
        name: "Nunavut",
        country_code: "CA",
        gst_rate_thousandths: Some(5000),
        pst_rate_thousandths: None,
        hst_rate_thousandths: None,
        effective_year: 2008,
        effective_month: 1,
        effective_day: 1,
        note: "Initial CRA seed",
    },
    ProvinceSeed {
        code: "ON",
        name: "Ontario",
        country_code: "CA",
        gst_rate_thousandths: None,
        pst_rate_thousandths: None,
        hst_rate_thousandths: Some(13000),
        effective_year: 2010,
        effective_month: 7,
        effective_day: 1,
        note: "Initial CRA seed",
    },
    ProvinceSeed {
        code: "PE",
        name: "Prince Edward Island",
        country_code: "CA",
        gst_rate_thousandths: None,
        pst_rate_thousandths: None,
        hst_rate_thousandths: Some(15000),
        effective_year: 2016,
        effective_month: 10,
        effective_day: 1,
        note: "Initial CRA seed",
    },
    ProvinceSeed {
        code: "QC",
        name: "Quebec",
        country_code: "CA",
        gst_rate_thousandths: Some(5000),
        pst_rate_thousandths: Some(9975),
        hst_rate_thousandths: None,
        effective_year: 2013,
        effective_month: 1,
        effective_day: 1,
        note: "Initial CRA seed",
    },
    ProvinceSeed {
        code: "SK",
        name: "Saskatchewan",
        country_code: "CA",
        gst_rate_thousandths: Some(5000),
        pst_rate_thousandths: Some(6000),
        hst_rate_thousandths: None,
        effective_year: 2017,
        effective_month: 3,
        effective_day: 23,
        note: "Initial CRA seed",
    },
    ProvinceSeed {
        code: "YT",
        name: "Yukon",
        country_code: "CA",
        gst_rate_thousandths: Some(5000),
        pst_rate_thousandths: None,
        hst_rate_thousandths: None,
        effective_year: 2008,
        effective_month: 1,
        effective_day: 1,
        note: "Initial CRA seed",
    },
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Provinces::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Provinces::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Provinces::Code)
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Provinces::Name)
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Provinces::CountryCode).text().not_null())
                    .col(ColumnDef::new(Provinces::GstRateThousandths).integer())
                    .col(ColumnDef::new(Provinces::PstRateThousandths).integer())
                    .col(ColumnDef::new(Provinces::HstRateThousandths).integer())
                    .col(ColumnDef::new(Provinces::EffectiveFrom).date().not_null())
                    .col(ColumnDef::new(Provinces::Note).text().not_null())
                    .col(
                        ColumnDef::new(Provinces::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Provinces::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        let db = manager.get_connection();

        for province in CANADIAN_PROVINCES {
            let select = Query::select()
                .expr(Expr::val(1))
                .from(Provinces::Table)
                .and_where(Expr::col(Provinces::Code).eq(province.code))
                .limit(1)
                .to_owned();

            let existing = db.query_one(&select).await?;

            if existing.is_some() {
                continue;
            }

            let effective_from = NaiveDate::from_ymd_opt(
                province.effective_year,
                province.effective_month,
                province.effective_day,
            )
            .ok_or_else(|| {
                DbErr::Migration(format!(
                    "Invalid seed date for province {}: {}-{}-{}",
                    province.code,
                    province.effective_year,
                    province.effective_month,
                    province.effective_day
                ))
            })?;

            let insert = Query::insert()
                .into_table(Provinces::Table)
                .columns([
                    Provinces::Code,
                    Provinces::Name,
                    Provinces::CountryCode,
                    Provinces::GstRateThousandths,
                    Provinces::PstRateThousandths,
                    Provinces::HstRateThousandths,
                    Provinces::EffectiveFrom,
                    Provinces::Note,
                ])
                .values_panic([
                    province.code.into(),
                    province.name.into(),
                    province.country_code.into(),
                    province.gst_rate_thousandths.into(),
                    province.pst_rate_thousandths.into(),
                    province.hst_rate_thousandths.into(),
                    effective_from.into(),
                    province.note.into(),
                ])
                .to_owned();

            db.execute(&insert).await?;
        }

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Provinces {
    Table,
    Id,
    Code,
    Name,
    CountryCode,
    GstRateThousandths,
    PstRateThousandths,
    HstRateThousandths,
    EffectiveFrom,
    Note,
    CreatedAt,
    UpdatedAt,
}
