// src/api/branches.rs: the branch calls
//
//   GET   /branches?status=active   list (all of them without ?status)
//   GET   /branches/{id}            read one
//   POST  /branches                 create (a new branch is always active)
//   PUT   /branches/{id}            replace the details (the status is not touched)
//   PATCH /branches/{id}/status     {"status": "inactive"} or {"status": "active"}
//
// There is no DELETE: a branch is never deleted, only made inactive.
// The JSON is only translated here; every rule (phone, postal code, province, unique number...)
// is the library's, the same as for any other caller.

use super::AppState;
use super::error::ApiError;
use crate::branch::{
    Branch, Location, create_branch, get_branch, list_branches, set_branch_status, update_branch,
};
use crate::customer::{Address, PhoneNumber, PostalCode};
use crate::errors::{QuoteError, QuoteResult};
use crate::province::Province;
use crate::status::Status;
use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// What a caller sends to create or replace a branch. Unknown fields are refused (so a caller
/// can't slip in a `status` or an `id` and think it worked).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BranchInput {
    number: String,
    name: String,
    address_line1: String,
    address_line2: Option<String>,
    city: String,
    /// The two-letter code: "SK", "AB".
    province: String,
    postal_code: String,
    phone: String,
    latitude: Option<f64>,
    longitude: Option<f64>,
}

impl BranchInput {
    /// Builds a `Branch` with the library's checked types; any rule that fails is returned.
    fn into_branch(self) -> QuoteResult<Branch> {
        let address = Address::new(
            self.address_line1,
            self.address_line2,
            self.city,
            PostalCode::parse(&self.postal_code)?,
            Province::from_code(&self.province)?,
        )?;
        let branch = Branch::new(
            self.number,
            self.name,
            address,
            PhoneNumber::parse(&self.phone)?,
        )?;
        match (self.latitude, self.longitude) {
            (Some(latitude), Some(longitude)) => {
                Ok(branch.with_location(Location::new(latitude, longitude)?))
            }
            (None, None) => Ok(branch),
            _ => Err(QuoteError::InvalidLocation), // only half a location
        }
    }
}

/// What a caller gets back.
#[derive(Serialize)]
pub(super) struct BranchView {
    id: String,
    number: String,
    name: String,
    address_line1: String,
    address_line2: Option<String>,
    city: String,
    province: String,
    country: String,
    postal_code: String,
    phone: String,
    latitude: Option<f64>,
    longitude: Option<f64>,
    status: &'static str,
}

impl From<&Branch> for BranchView {
    fn from(branch: &Branch) -> Self {
        let address = branch.address();
        BranchView {
            id: branch.id().map(|id| id.to_string()).unwrap_or_default(),
            number: branch.number().to_string(),
            name: branch.name().to_string(),
            address_line1: address.line1().to_string(),
            address_line2: address.line2().map(str::to_string),
            city: address.city().to_string(),
            province: branch.province().code().to_string(),
            country: branch.country().code().to_string(),
            postal_code: address.postal_code().to_string(),
            phone: branch.phone().to_string(),
            latitude: branch.location().map(Location::latitude),
            longitude: branch.location().map(Location::longitude),
            status: branch.status().label(),
        }
    }
}

#[derive(Deserialize)]
pub(super) struct ListQuery {
    status: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StatusInput {
    status: String,
}

/// A body that isn't the expected JSON is one plain answer, not serde's wording.
fn body<T>(input: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    input
        .map(|Json(value)| value)
        .map_err(|_| ApiError::BadBody)
}

/// An id that isn't a UUID can't be a branch: the same answer as an id nobody has.
fn branch_id(text: &str) -> QuoteResult<Uuid> {
    Uuid::parse_str(text).map_err(|_| QuoteError::UnknownBranch)
}

pub(super) async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<BranchView>>, ApiError> {
    let status = query
        .status
        .as_deref()
        .map(Status::from_label)
        .transpose()?;
    let branches = list_branches(&state.db, status).await?;
    Ok(Json(branches.iter().map(BranchView::from).collect()))
}

pub(super) async fn read(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<BranchView>, ApiError> {
    let branch = get_branch(&state.db, branch_id(&id)?).await?;
    Ok(Json(BranchView::from(&branch)))
}

pub(super) async fn create(
    State(state): State<AppState>,
    input: Result<Json<BranchInput>, JsonRejection>,
) -> Result<(StatusCode, Json<BranchView>), ApiError> {
    let branch = body(input)?.into_branch()?;
    let saved = create_branch(&state.db, &branch).await?;
    Ok((StatusCode::CREATED, Json(BranchView::from(&saved))))
}

pub(super) async fn replace(
    State(state): State<AppState>,
    Path(id): Path<String>,
    input: Result<Json<BranchInput>, JsonRejection>,
) -> Result<Json<BranchView>, ApiError> {
    let id = branch_id(&id)?;
    let new_details = body(input)?.into_branch()?;
    // The details are replaced, the status is not: a closed branch must not reopen by accident.
    let current = get_branch(&state.db, id).await?;
    let updated = update_branch(
        &state.db,
        &new_details.with_id(id).with_status(current.status()),
    )
    .await?;
    Ok(Json(BranchView::from(&updated)))
}

pub(super) async fn change_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    input: Result<Json<StatusInput>, JsonRejection>,
) -> Result<Json<BranchView>, ApiError> {
    let id = branch_id(&id)?;
    let status = Status::from_label(&body(input)?.status)?;
    let branch = set_branch_status(&state.db, id, status).await?;
    Ok(Json(BranchView::from(&branch)))
}
