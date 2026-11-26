// src/api/branches.rs: the branch calls
//
//   GET   /branches?status=active   list (all of them without ?status)
//   GET   /branches/{id}            read one
//   POST  /branches                 create (a new branch is always active)
//   PUT   /branches/{id}            replace ALL the details (the status is not touched)
//   PATCH /branches/{id}            change only the details that are sent (the rest stays as it is)
//   PATCH /branches/{id}/status     {"status": "inactive"} or {"status": "active"}
//
// There is no DELETE: a branch is never deleted, only made inactive.
// The JSON is only translated here; every rule (phone, postal code, province, unique number...)
// is the library's, the same as for any other caller.

use super::AppState;
use super::error::{ApiError, ErrorBody};
use super::request::{StatusInput, body, changes, merge};
use crate::branch::{
    Branch, create_branch, get_branch, list_branches, set_branch_status, update_branch,
};
use crate::common::{Address, Location, PhoneNumber, PostalCode};
use crate::errors::{AppError, AppResult};
use crate::province_country::ProvinceCountry;
use crate::status::Status;
use axum::Json;
use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// What a caller sends to create or replace a branch. Unknown fields are refused (so a caller
/// can't slip in a `status` or an `id` and think it worked).
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(example = json!({
    "number": "001", "name": "Regina Branch",
    "address_line1": "1 Industrial Rd", "address_line2": "Unit 4", "city": "Regina",
    "province_code": "SK", "postal_code": "S4P 3Y2", "phone": "306-555-0100",
    "latitude": 50.4452, "longitude": -104.6189
}))]
pub(super) struct BranchInput {
    number: String,
    name: String,
    address_line1: String,
    address_line2: Option<String>,
    city: String,
    /// The two-letter code. Only "SK" and "AB" are known so far.
    province_code: String,
    postal_code: String,
    /// Any usual way of writing a 10-digit phone number.
    phone: String,
    /// Give both latitude and longitude, or neither.
    latitude: Option<f64>,
    longitude: Option<f64>,
}

impl BranchInput {
    /// Builds a `Branch` with the library's checked types; any rule that fails is returned.
    fn into_branch(self) -> AppResult<Branch> {
        let address = Address::new(
            self.address_line1,
            self.address_line2,
            self.city,
            PostalCode::parse(&self.postal_code)?,
            ProvinceCountry::from_province_code(&self.province_code)?,
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
            _ => Err(AppError::InvalidLocation), // only half a location
        }
    }
}

/// What a caller gets back.
#[derive(Serialize, ToSchema)]
pub(super) struct BranchView {
    /// Given by the database when the branch is created.
    id: String,
    number: String,
    name: String,
    address_line1: String,
    address_line2: Option<String>,
    city: String,
    province_code: String,
    country: String,
    postal_code: String,
    phone: String,
    latitude: Option<f64>,
    longitude: Option<f64>,
    /// "active" or "inactive".
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
            province_code: address.province_country().province_code().to_string(),
            country: address.province_country().country_code().to_string(),
            postal_code: address.postal_code().to_string(),
            phone: branch.phone().to_string(),
            latitude: branch.location().map(Location::latitude),
            longitude: branch.location().map(Location::longitude),
            status: branch.status().label(),
        }
    }
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct ListQuery {
    /// Only the branches with this status: "active" or "inactive". Leave out for all.
    status: Option<String>,
}

/// Only for the docs page: any of the fields of a branch. (The call itself reads the body as a
/// plain object and checks it with `BranchInput`.)
#[derive(ToSchema)]
#[schema(example = json!({"phone": "306-555-0199", "address_line2": null}))]
#[allow(dead_code)]
pub(super) struct BranchChanges {
    number: Option<String>,
    name: Option<String>,
    address_line1: Option<String>,
    address_line2: Option<String>,
    city: Option<String>,
    province_code: Option<String>,
    postal_code: Option<String>,
    phone: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
}

/// An id that isn't a UUID can't be a branch: the same answer as an id nobody has.
fn branch_id(text: &str) -> AppResult<Uuid> {
    Uuid::parse_str(text).map_err(|_| AppError::UnknownBranch)
}

/// List the branches, ordered by number.
#[utoipa::path(get, path = "/branches", operation_id = "list_branches", tag = "branches", params(ListQuery),
    responses(
        (status = 200, description = "The branches.", body = Vec<BranchView>),
        (status = 400, description = "The status is not 'active' or 'inactive'.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn list(
    State(state): State<AppState>,
    query: Result<Query<ListQuery>, QueryRejection>,
) -> Result<Json<Vec<BranchView>>, ApiError> {
    let Query(query) = query.map_err(|_| ApiError::BadQuery)?;
    let status = query
        .status
        .as_deref()
        .map(Status::from_label)
        .transpose()?;
    let branches = list_branches(&state.db, status).await?;
    Ok(Json(branches.iter().map(BranchView::from).collect()))
}

/// Read one branch.
#[utoipa::path(get, path = "/branches/{id}", operation_id = "read_branch", tag = "branches",
    params(("id" = String, Path, description = "The branch id.")),
    responses(
        (status = 200, description = "The branch.", body = BranchView),
        (status = 404, description = "No branch has that id.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn read(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<BranchView>, ApiError> {
    let branch = get_branch(&state.db, branch_id(&id)?).await?;
    Ok(Json(BranchView::from(&branch)))
}

/// Create a branch. A new branch is always active.
#[utoipa::path(post, path = "/branches", operation_id = "create_branch", tag = "branches", request_body = BranchInput,
    responses(
        (status = 201, description = "The branch as saved, with its id.", body = BranchView),
        (status = 400, description = "The body is wrong, or a rule refused a value.", body = ErrorBody),
        (status = 409, description = "Another branch already has that number.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn create(
    State(state): State<AppState>,
    input: Result<Json<BranchInput>, JsonRejection>,
) -> Result<(StatusCode, Json<BranchView>), ApiError> {
    let branch = body(input)?.into_branch()?;
    let saved = create_branch(&state.db, &branch).await?;
    Ok((StatusCode::CREATED, Json(BranchView::from(&saved))))
}

/// Replace ALL of a branch's details (every required field must be sent). Its status is not changed.
/// To change only some, use PATCH.
#[utoipa::path(put, path = "/branches/{id}", operation_id = "replace_branch", tag = "branches", request_body = BranchInput,
    params(("id" = String, Path, description = "The branch id.")),
    responses(
        (status = 200, description = "The branch as saved.", body = BranchView),
        (status = 400, description = "The body is wrong, or a rule refused a value.", body = ErrorBody),
        (status = 404, description = "No branch has that id.", body = ErrorBody),
        (status = 409, description = "Another branch already has that number.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
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

/// Change only the details that are sent; the rest stays as it is. Send `null` to clear an optional
/// field (`address_line2`, or both `latitude` and `longitude`). The status is not changed here.
#[utoipa::path(patch, path = "/branches/{id}", operation_id = "change_branch", tag = "branches", request_body = BranchChanges,
    params(("id" = String, Path, description = "The branch id.")),
    responses(
        (status = 200, description = "The branch as saved.", body = BranchView),
        (status = 400, description = "The body is empty or wrong, or a rule refused a value.", body = ErrorBody),
        (status = 404, description = "No branch has that id.", body = ErrorBody),
        (status = 409, description = "Another branch already has that number.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn change(
    State(state): State<AppState>,
    Path(id): Path<String>,
    input: Result<Json<Map<String, Value>>, JsonRejection>,
) -> Result<Json<BranchView>, ApiError> {
    let id = branch_id(&id)?;
    let changes = changes(input)?;
    let current = get_branch(&state.db, id).await?;
    let branch = merge::<BranchInput>(&BranchView::from(&current), changes)?.into_branch()?;
    let updated =
        update_branch(&state.db, &branch.with_id(id).with_status(current.status())).await?;
    Ok(Json(BranchView::from(&updated)))
}

/// Make a branch active or inactive. (A branch is never deleted.)
#[utoipa::path(patch, path = "/branches/{id}/status", operation_id = "change_branch_status", tag = "branches", request_body = StatusInput,
    params(("id" = String, Path, description = "The branch id.")),
    responses(
        (status = 200, description = "The branch with its new status.", body = BranchView),
        (status = 400, description = "The status is not 'active' or 'inactive'.", body = ErrorBody),
        (status = 404, description = "No branch has that id.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
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
