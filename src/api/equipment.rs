// src/api/equipment.rs: the equipment (machine) calls
//
//   GET    /equipment?status=&customer_id=&unowned=&serial_number=&limit=&offset=   a page of machines
//   GET    /equipment/{id}          read one
//   POST   /equipment               create (always active; an owner is optional)
//   PUT    /equipment/{id}          replace ALL the details (not the owner, not the status)
//   PATCH  /equipment/{id}          change only the details that are sent
//   DELETE /equipment/{id}          ARCHIVE: the machine is kept, hidden from new quotes
//   PATCH  /equipment/{id}/status   {"status": "inactive"} archives, {"status": "active"} restores
//   PATCH  /equipment/{id}/owner    {"customer_id": "<id>"} gives it to a customer, null to nobody
//
// A machine is never erased: DELETE means archive. A machine may have no owner yet; a sale is a
// change of owner on the same record (so one machine stays one record: the serial number is
// unique). A serial number or a plate identifies one real vehicle: nothing from a machine reaches
// a log (see `claude/logging-rules.md`), and only made-up machines may be saved until real login
// exists. The JSON is only translated here; every rule is the library's.

use super::AppState;
use super::error::{ApiError, ErrorBody};
use super::request::{StatusInput, body, changes, merge, page_size};
use crate::equipment::{
    Equipment, EquipmentFilter, EquipmentType, Identification, Meter, MeterType, OwnerFilter,
    create_equipment, get_equipment, list_equipment, set_equipment_owner, set_equipment_status,
    update_equipment,
};
use crate::errors::{QuoteError, QuoteResult};
use crate::status::Status;
use axum::Json;
use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// What a caller sends to replace a machine's details (and, with only some fields, to change
/// them). Unknown fields are refused: a caller can't slip in a `status`, an `id` or a
/// `customer_id` here and think it worked.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(example = json!({
    "make": "John Deere", "model": "8R 310", "model_year": 2021,
    "serial_number": "1RW8R310ABC000001", "plate": "TR-1234",
    "meter_reading": 1250.5, "meter_type": "hr", "equipment_type": "tractor"
}))]
pub(super) struct EquipmentInput {
    make: String,
    model: String,
    model_year: Option<i64>,
    /// Kept in capitals. A serial number is unique: the same machine is never saved twice.
    serial_number: Option<String>,
    /// A serial number or a plate is required. A plate is not unique (plates move).
    plate: Option<String>,
    /// Zero or more; leave out when it wasn't read.
    meter_reading: Option<f64>,
    /// "km", "mi" or "hr".
    meter_type: String,
    /// "car", "truck", "tractor", "trailer" or "other".
    equipment_type: String,
}

impl EquipmentInput {
    /// Builds an `Equipment` with the library's checked types; any rule that fails is returned.
    fn into_equipment(self) -> QuoteResult<Equipment> {
        Equipment::new(
            self.make,
            self.model,
            self.model_year,
            Identification::new(self.serial_number, self.plate)?,
            Meter::new(self.meter_reading, MeterType::from_code(&self.meter_type)?)?,
            EquipmentType::from_label(&self.equipment_type)?,
        )
    }
}

/// Only for the docs page: what a caller sends to create a machine (the details, and optionally
/// an owner). The call reads the body as a plain object and checks it with `EquipmentInput`.
#[derive(ToSchema)]
#[schema(example = json!({
    "make": "John Deere", "model": "8R 310", "model_year": 2021,
    "serial_number": "1RW8R310ABC000001", "meter_reading": 1250.5, "meter_type": "hr",
    "equipment_type": "tractor", "customer_id": null
}))]
#[allow(dead_code)]
pub(super) struct NewEquipment {
    make: String,
    model: String,
    model_year: Option<i64>,
    serial_number: Option<String>,
    plate: Option<String>,
    meter_reading: Option<f64>,
    meter_type: String,
    equipment_type: String,
    /// The owner, if already a customer. Leave out or null for a machine with no owner yet.
    customer_id: Option<String>,
}

/// Only for the docs page: any of the details of a machine.
#[derive(ToSchema)]
#[schema(example = json!({"meter_reading": 1300.0, "plate": null}))]
#[allow(dead_code)]
pub(super) struct EquipmentChanges {
    make: Option<String>,
    model: Option<String>,
    model_year: Option<i64>,
    serial_number: Option<String>,
    plate: Option<String>,
    meter_reading: Option<f64>,
    meter_type: Option<String>,
    equipment_type: Option<String>,
}

/// Only for the docs page: the body of the owner call.
#[derive(ToSchema)]
#[schema(example = json!({"customer_id": null}))]
#[allow(dead_code)]
pub(super) struct OwnerInput {
    /// The new owner's id, or null for nobody. The key must be there.
    customer_id: Option<String>,
}

/// What a caller gets back.
#[derive(Serialize, ToSchema)]
pub(super) struct EquipmentView {
    /// Given by the database when the machine is created.
    id: String,
    /// The owner, or null while the machine has none.
    customer_id: Option<String>,
    make: String,
    model: String,
    model_year: Option<i64>,
    serial_number: Option<String>,
    plate: Option<String>,
    meter_reading: Option<f64>,
    meter_type: &'static str,
    /// "car", "truck", "tractor", "trailer" or "other" (the same words a caller sends).
    equipment_type: String,
    /// Where the details came from, e.g. "entered by hand".
    source: String,
    /// "active", or "inactive" when archived.
    status: &'static str,
}

impl From<&Equipment> for EquipmentView {
    fn from(equipment: &Equipment) -> Self {
        let identification = equipment.identification();
        EquipmentView {
            id: equipment.id().map(|id| id.to_string()).unwrap_or_default(),
            customer_id: equipment.customer_id().map(|id| id.to_string()),
            make: equipment.make().to_string(),
            model: equipment.model().to_string(),
            model_year: equipment.model_year(),
            serial_number: identification.serial_number().map(str::to_string),
            plate: identification.plate().map(str::to_string),
            meter_reading: equipment.meter().reading(),
            meter_type: equipment.meter().meter_type().code(),
            equipment_type: equipment.equipment_type().label().to_lowercase(),
            source: equipment.source().to_string(),
            status: equipment.status().label(),
        }
    }
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct ListQuery {
    /// Only machines with this status: "active" or "inactive" (archived). Leave out for all.
    status: Option<String>,
    /// Only the machines of this customer.
    customer_id: Option<String>,
    /// `true`: only machines with no owner yet. Not together with `customer_id`.
    unowned: Option<bool>,
    /// Find a machine by its serial number (any case).
    serial_number: Option<String>,
    /// How many machines to return (1 to 200). Default 50.
    limit: Option<u64>,
    /// How many machines to skip (for the next page). Default 0.
    offset: Option<u64>,
}

impl ListQuery {
    /// What the library should narrow the list to. A query that makes no sense is refused.
    fn filter(&self) -> Result<EquipmentFilter, ApiError> {
        let owner = match (&self.customer_id, self.unowned) {
            (Some(_), Some(true)) => return Err(ApiError::BadQuery),
            (Some(id), _) => {
                OwnerFilter::Owned(Uuid::parse_str(id).map_err(|_| ApiError::BadQuery)?)
            }
            (None, Some(true)) => OwnerFilter::Unowned,
            (None, _) => OwnerFilter::Any,
        };
        Ok(EquipmentFilter {
            status: self.status.as_deref().map(Status::from_label).transpose()?,
            owner,
            serial_number: self.serial_number.clone(),
        })
    }
}

/// An id that isn't a UUID can't be a machine: the same answer as an id nobody has.
fn equipment_id(text: &str) -> QuoteResult<Uuid> {
    Uuid::parse_str(text).map_err(|_| QuoteError::UnknownEquipment)
}

/// An owner id that isn't a UUID can't be a customer.
fn owner_id(text: &str) -> QuoteResult<Uuid> {
    Uuid::parse_str(text).map_err(|_| QuoteError::UnknownCustomer)
}

/// List a page of machines, ordered by make, model and serial number.
#[utoipa::path(get, path = "/equipment", operation_id = "list_equipment", tag = "equipment", params(ListQuery),
    responses(
        (status = 200, description = "A page of machines.", body = Vec<EquipmentView>),
        (status = 400, description = "The status is not 'active' or 'inactive', or the query is not valid.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn list(
    State(state): State<AppState>,
    query: Result<Query<ListQuery>, QueryRejection>,
) -> Result<Json<Vec<EquipmentView>>, ApiError> {
    let Query(query) = query.map_err(|_| ApiError::BadQuery)?;
    let machines = list_equipment(
        &state.db,
        &query.filter()?,
        page_size(query.limit),
        query.offset.unwrap_or(0),
    )
    .await?;
    Ok(Json(machines.iter().map(EquipmentView::from).collect()))
}

/// Read one machine.
#[utoipa::path(get, path = "/equipment/{id}", operation_id = "read_equipment", tag = "equipment",
    params(("id" = String, Path, description = "The machine's id.")),
    responses(
        (status = 200, description = "The machine.", body = EquipmentView),
        (status = 404, description = "No machine has that id.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn read(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<EquipmentView>, ApiError> {
    let machine = get_equipment(&state.db, equipment_id(&id)?).await?;
    Ok(Json(EquipmentView::from(&machine)))
}

/// Create a machine. It is always active. The owner is optional: leave `customer_id` out (or
/// null) when the customer isn't known yet and give one later with the owner call.
#[utoipa::path(post, path = "/equipment", operation_id = "create_equipment", tag = "equipment", request_body = NewEquipment,
    responses(
        (status = 201, description = "The machine as saved, with its id.", body = EquipmentView),
        (status = 400, description = "The body is wrong, or a rule refused a value.", body = ErrorBody),
        (status = 404, description = "The owner given is not a customer.", body = ErrorBody),
        (status = 409, description = "Another machine already has that serial number, or the owner is inactive.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn create(
    State(state): State<AppState>,
    input: Result<Json<Map<String, Value>>, JsonRejection>,
) -> Result<(StatusCode, Json<EquipmentView>), ApiError> {
    let mut fields = body(input)?;
    // The owner is the one field that isn't a detail: take it out, check the rest as usual.
    let owner = match fields.remove("customer_id") {
        None | Some(Value::Null) => None,
        Some(Value::String(text)) => Some(owner_id(&text)?),
        Some(_) => return Err(ApiError::BadBody),
    };
    let details: EquipmentInput =
        serde_json::from_value(Value::Object(fields)).map_err(|_| ApiError::BadBody)?;
    let mut machine = details.into_equipment()?;
    if let Some(owner) = owner {
        machine = machine.with_customer_id(owner);
    }
    let saved = create_equipment(&state.db, &machine).await?;
    Ok((StatusCode::CREATED, Json(EquipmentView::from(&saved))))
}

/// Replace ALL of a machine's details (every required field must be sent). Its owner and status
/// are not changed. To change only some details, use PATCH.
#[utoipa::path(put, path = "/equipment/{id}", operation_id = "replace_equipment", tag = "equipment", request_body = EquipmentInput,
    params(("id" = String, Path, description = "The machine's id.")),
    responses(
        (status = 200, description = "The machine as saved.", body = EquipmentView),
        (status = 400, description = "The body is wrong, or a rule refused a value.", body = ErrorBody),
        (status = 404, description = "No machine has that id.", body = ErrorBody),
        (status = 409, description = "Another machine already has that serial number.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn replace(
    State(state): State<AppState>,
    Path(id): Path<String>,
    input: Result<Json<EquipmentInput>, JsonRejection>,
) -> Result<Json<EquipmentView>, ApiError> {
    let id = equipment_id(&id)?;
    let new_details = body(input)?.into_equipment()?;
    let updated = update_equipment(&state.db, &new_details.with_id(id)).await?;
    Ok(Json(EquipmentView::from(&updated)))
}

/// Change only the details that are sent; the rest stays as it is. Send `null` to clear an
/// optional detail (`model_year`, `serial_number`, `plate`, `meter_reading`). The owner and the
/// status are not changed here.
#[utoipa::path(patch, path = "/equipment/{id}", operation_id = "change_equipment", tag = "equipment", request_body = EquipmentChanges,
    params(("id" = String, Path, description = "The machine's id.")),
    responses(
        (status = 200, description = "The machine as saved.", body = EquipmentView),
        (status = 400, description = "The body is empty or wrong, or a rule refused a value.", body = ErrorBody),
        (status = 404, description = "No machine has that id.", body = ErrorBody),
        (status = 409, description = "Another machine already has that serial number.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn change(
    State(state): State<AppState>,
    Path(id): Path<String>,
    input: Result<Json<Map<String, Value>>, JsonRejection>,
) -> Result<Json<EquipmentView>, ApiError> {
    let id = equipment_id(&id)?;
    let changes = changes(input)?;
    let current = get_equipment(&state.db, id).await?;
    let machine =
        merge::<EquipmentInput>(&EquipmentView::from(&current), changes)?.into_equipment()?;
    let updated = update_equipment(&state.db, &machine.with_id(id)).await?;
    Ok(Json(EquipmentView::from(&updated)))
}

/// "Delete" a machine: it is **archived, never erased**. It disappears from lists (unless you
/// ask for inactive ones) and can't go on a new quote; old quotes and everything else that
/// points at it are untouched. Restore it with the status call.
#[utoipa::path(delete, path = "/equipment/{id}", operation_id = "archive_equipment", tag = "equipment",
    params(("id" = String, Path, description = "The machine's id.")),
    responses(
        (status = 200, description = "The machine, now inactive.", body = EquipmentView),
        (status = 404, description = "No machine has that id.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn archive(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<EquipmentView>, ApiError> {
    let machine = set_equipment_status(&state.db, equipment_id(&id)?, Status::Inactive).await?;
    Ok(Json(EquipmentView::from(&machine)))
}

/// Archive or restore a machine.
#[utoipa::path(patch, path = "/equipment/{id}/status", operation_id = "change_equipment_status", tag = "equipment", request_body = StatusInput,
    params(("id" = String, Path, description = "The machine's id.")),
    responses(
        (status = 200, description = "The machine with its new status.", body = EquipmentView),
        (status = 400, description = "The status is not 'active' or 'inactive'.", body = ErrorBody),
        (status = 404, description = "No machine has that id.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn change_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    input: Result<Json<StatusInput>, JsonRejection>,
) -> Result<Json<EquipmentView>, ApiError> {
    let id = equipment_id(&id)?;
    let status = Status::from_label(&body(input)?.status)?;
    let machine = set_equipment_status(&state.db, id, status).await?;
    Ok(Json(EquipmentView::from(&machine)))
}

/// Give a machine to a customer (a sale moves it from one customer to another), or to nobody
/// with `{"customer_id": null}`. The key `customer_id` must be in the body. Only the current
/// owner is kept.
#[utoipa::path(patch, path = "/equipment/{id}/owner", operation_id = "change_equipment_owner", tag = "equipment", request_body = OwnerInput,
    params(("id" = String, Path, description = "The machine's id.")),
    responses(
        (status = 200, description = "The machine with its new owner.", body = EquipmentView),
        (status = 400, description = "The body is not exactly {\"customer_id\": id or null}.", body = ErrorBody),
        (status = 404, description = "No machine has that id, or the owner is not a customer.", body = ErrorBody),
        (status = 409, description = "The new owner is an inactive customer.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn change_owner(
    State(state): State<AppState>,
    Path(id): Path<String>,
    input: Result<Json<Map<String, Value>>, JsonRejection>,
) -> Result<Json<EquipmentView>, ApiError> {
    let id = equipment_id(&id)?;
    let fields = body(input)?;
    // Exactly one key, so an empty body can never clear the owner by accident.
    let owner = match (fields.len(), fields.get("customer_id")) {
        (1, Some(Value::Null)) => None,
        (1, Some(Value::String(text))) => Some(owner_id(text)?),
        _ => return Err(ApiError::BadBody),
    };
    let machine = set_equipment_owner(&state.db, id, owner).await?;
    Ok(Json(EquipmentView::from(&machine)))
}
