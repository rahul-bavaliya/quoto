// src/api/provinces.rs: the province and tax-rate endpoints.
//
// GET   /provinces             -> list all provinces and their tax rates
// GET   /provinces/{code}      -> read one province's tax record by code
// PATCH /provinces/{code}/rates -> update a province's tax rates (never deleted)

use axum::Json;
use axum::extract::{Path, State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::AppState;
use super::error::{ApiError, ErrorBody};
use crate::province_country::{
    Province, ProvinceTaxRecord, TaxRateInfo, change_rates as update_rates, get_province_tax_rates,
    list_province_tax_rates,
};

#[derive(Debug, Serialize, ToSchema)]
pub(super) struct TaxRatesResponse {
    pub gst_milli_pct: Option<i32>,
    pub pst_milli_pct: Option<i32>,
    pub hst_milli_pct: Option<i32>,
    pub total_milli_pct: i32,
}

impl From<&TaxRateInfo> for TaxRatesResponse {
    fn from(rates: &TaxRateInfo) -> Self {
        Self {
            gst_milli_pct: rates.gst_milli_pct,
            pst_milli_pct: rates.pst_milli_pct,
            hst_milli_pct: rates.hst_milli_pct,
            total_milli_pct: rates.total_milli_pct(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub(super) struct ProvinceResponse {
    pub code: String,
    pub name: String,
    pub country_code: String,
    pub tax_rates: TaxRatesResponse,
    pub note: Option<String>,
    pub effective_date: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

impl From<ProvinceTaxRecord> for ProvinceResponse {
    fn from(rec: ProvinceTaxRecord) -> Self {
        Self {
            code: rec.code,
            name: rec.name,
            country_code: rec.country.code().to_string(),
            tax_rates: TaxRatesResponse::from(&rec.rates),
            note: rec.note,
            effective_date: rec.effective_date,
            updated_at: rec.updated_at,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct ChangeRatesBody {
    pub gst_milli_pct: Option<i32>,
    pub pst_milli_pct: Option<i32>,
    pub hst_milli_pct: Option<i32>,
    pub note: String,
    pub effective_date: DateTime<Utc>,
}

/// Lists all provinces and their current tax rates.
#[utoipa::path(
    get,
    path = "/provinces",
    responses(
        (status = 200, description = "List all provinces and tax rates", body = Vec<ProvinceResponse>),
        (status = 401, description = "Missing or invalid API key", body = ErrorBody),
        (status = 503, description = "Database unavailable", body = ErrorBody)
    ),
    security(("api_key" = []))
)]
pub(super) async fn list(
    State(state): State<AppState>,
) -> Result<Json<Vec<ProvinceResponse>>, ApiError> {
    let records = list_province_tax_rates(&state.db).await?;
    Ok(Json(
        records.into_iter().map(ProvinceResponse::from).collect(),
    ))
}

/// Reads one province and its tax rates by province code (e.g. "SK", "ON").
#[utoipa::path(
    get,
    path = "/provinces/{code}",
    params(
        ("code" = String, Path, description = "Province or territory code (e.g. SK, ON)")
    ),
    responses(
        (status = 200, description = "Province tax record", body = ProvinceResponse),
        (status = 400, description = "Invalid province code", body = ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = ErrorBody),
        (status = 404, description = "Province not found", body = ErrorBody),
        (status = 503, description = "Database unavailable", body = ErrorBody)
    ),
    security(("api_key" = []))
)]
pub(super) async fn read(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<ProvinceResponse>, ApiError> {
    let record = get_province_tax_rates(&state.db, &code).await?;
    Ok(Json(ProvinceResponse::from(record)))
}

/// Updates a province's tax rates, enforcing rate/combination, note, and effective-date rules.
#[utoipa::path(
    patch,
    path = "/provinces/{code}/rates",
    params(
        ("code" = String, Path, description = "Province or territory code (e.g. SK, ON)")
    ),
    request_body = ChangeRatesBody,
    responses(
        (status = 200, description = "Updated province tax record", body = ProvinceResponse),
        (status = 400, description = "Invalid tax rates, combination, note, or effective date", body = ErrorBody),
        (status = 401, description = "Missing or invalid API key", body = ErrorBody),
        (status = 404, description = "Province not found", body = ErrorBody),
        (status = 503, description = "Database unavailable", body = ErrorBody)
    ),
    security(("api_key" = []))
)]
pub(super) async fn change_rates(
    State(state): State<AppState>,
    Path(code): Path<String>,
    Json(body): Json<ChangeRatesBody>,
) -> Result<Json<ProvinceResponse>, ApiError> {
    let province = Province::from_code(&code)?;
    let rates = TaxRateInfo::new(body.gst_milli_pct, body.pst_milli_pct, body.hst_milli_pct)?;
    let updated = update_rates(&state.db, province, rates, &body.note, body.effective_date).await?;
    Ok(Json(ProvinceResponse::from(updated)))
}
