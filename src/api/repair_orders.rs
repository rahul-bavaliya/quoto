// src/api/repair_orders.rs: the repair order calls
//
//   POST  /quotes/{id}/repair-order   {"meter_reading": 1250.5}   open a repair order from an accepted quote
//   GET   /repair-orders?customer_id=&branch_id=&equipment_id=&status=&limit=&offset=   a page, newest first
//   GET   /repair-orders/{id}         read one (with the quote's lines and totals, its version, and an ETag)
//   PATCH /repair-orders/{id}         the due date and the three paragraphs: {"due_date": "2026-12-01",
//                                     "complaint": "...", "cause": "...", "correction": "..."} (any of them; null clears)
//   POST  /repair-orders/{id}/start     the work begins
//   POST  /repair-orders/{id}/complete  the work is done
//   POST  /repair-orders/{id}/cancel    {"reason": "..."}  the work will not be done, and why
//
// A repair order is made only from an accepted quote, and a quote has at most one. The work, the
// prices and the people are the quote's: they are read from it, never copied, so they can't
// disagree. The order adds what the quote does not know: its own number, status, due date, when
// it started and ended, and the meter reading on the day the machine came in. It also carries
// three optional paragraphs written by people: the complaint, the cause and the correction.
//
// It has a `version` and an `ETag`, and a call that changes it may send `If-Match`, exactly like
// a quote (`409 REPAIR_ORDER_CHANGED_MEANWHILE`). Nothing from a repair order reaches a log, and the
// paragraphs least of all.

use super::AppState;
use super::branches::BranchView;
use super::customers::CustomerView;
use super::equipment::EquipmentView;
use super::error::{ApiError, ErrorBody};
use super::quotes::{LineView, TotalsView, if_match, now};
use super::request::{body, page_size};
use crate::errors::{QuoteError, QuoteResult};
use crate::{
    NewRepairOrder, RepairOrder, RepairOrderFilter, RepairOrderStatus, create_repair_order,
    get_repair_order, list_repair_orders, save_repair_order,
};
use axum::Json;
use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderName, StatusCode, header};
use axum::response::IntoResponse;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// The body of "open a repair order".
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(example = json!({"meter_reading": 1250.5}))]
pub(super) struct OpenRepairOrderInput {
    /// What the meter says today (hours or km, never negative). Leave it out, or send `null`, to
    /// use the reading on the quote's machine; with neither, the answer is 400 METER_READING_REQUIRED.
    meter_reading: Option<f64>,
}

/// The body of "cancel".
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(example = json!({"reason": "The part is discontinued"}))]
pub(super) struct CancelRepairOrderInput {
    /// Why (not blank, at most 500 characters). Never logged.
    reason: String,
}

/// The body of the PATCH: any of these (at least one); what is left out stays; `null` clears.
#[derive(ToSchema)]
#[schema(example = json!({"due_date": "2026-12-01", "complaint": "Hydraulics are slow when warm.", "cause": "Worn pump seal.", "correction": "Replaced the seal and refilled the oil."}))]
#[allow(dead_code)]
pub(super) struct RepairOrderChanges {
    /// `YYYY-MM-DD`. Not before the day the order was opened.
    #[schema(value_type = Option<String>, format = Date)]
    due_date: Option<NaiveDate>,
    /// What the customer says is wrong. Free text, at most 4000 characters (new lines are fine);
    /// blank or `null` clears it. Never logged.
    complaint: Option<String>,
    /// What the technician found to be the cause. Same rules as the complaint.
    cause: Option<String>,
    /// What was done to put it right. Same rules as the complaint.
    correction: Option<String>,
}

/// What the meter said when the machine came in.
#[derive(Serialize, ToSchema)]
pub(super) struct RepairOrderMeterView {
    reading: f64,
    /// "hr" or "km".
    #[serde(rename = "type")]
    meter_type: &'static str,
}

/// What a caller gets back.
#[derive(Serialize, ToSchema)]
pub(super) struct RepairOrderView {
    id: String,
    /// 0 when the order is opened, +1 with every change. Also sent as the `ETag` header.
    version: i32,
    /// `RO-<year>-<8 digits>`.
    number: String,
    /// "open", "in_progress", "completed" or "cancelled".
    status: &'static str,
    /// Only on a cancelled order.
    cancellation_reason: Option<String>,
    /// The shop's local time, `YYYY-MM-DDTHH:MM:SS`.
    created_at: String,
    /// `YYYY-MM-DD`. Starts as the quote's promise date, if it had one.
    due_date: Option<String>,
    started_at: Option<String>,
    /// When it was completed or cancelled.
    closed_at: Option<String>,
    /// What the customer says is wrong. Optional free text.
    complaint: Option<String>,
    /// What the technician found to be the cause. Optional free text.
    cause: Option<String>,
    /// What was done to put it right. Optional free text.
    correction: Option<String>,
    meter: RepairOrderMeterView,
    /// The accepted quote this order came from.
    quote_id: String,
    quote_number: String,
    /// The branch, customer and machine as they were on the quote.
    branch: BranchView,
    customer: Option<CustomerView>,
    equipment: Option<EquipmentView>,
    /// The quote's lines, which are the work to do.
    items: Vec<LineView>,
    totals: TotalsView,
}

fn text_time(time: chrono::NaiveDateTime) -> String {
    time.format("%Y-%m-%dT%H:%M:%S").to_string()
}

impl RepairOrderView {
    fn new(order: &RepairOrder) -> Self {
        let quote = order.quote();
        let cancellation_reason = match order.status() {
            RepairOrderStatus::Cancelled { reason } => Some(reason.clone()),
            _ => None,
        };
        RepairOrderView {
            id: order.id().map(|id| id.to_string()).unwrap_or_default(),
            version: order.version(),
            number: order.number().to_string(),
            status: order.status().label(),
            cancellation_reason,
            created_at: text_time(order.created_at()),
            due_date: order.due_date().map(|date| date.to_string()),
            started_at: order.started_at().map(text_time),
            closed_at: order.closed_at().map(text_time),
            complaint: order.complaint().map(str::to_string),
            cause: order.cause().map(str::to_string),
            correction: order.correction().map(str::to_string),
            meter: RepairOrderMeterView {
                reading: order.meter_reading(),
                meter_type: order.meter_type().code(),
            },
            quote_id: quote.id().map(|id| id.to_string()).unwrap_or_default(),
            quote_number: quote.number().to_string(),
            branch: BranchView::from(quote.branch()),
            customer: quote.customer().map(CustomerView::from),
            equipment: quote.equipment().map(EquipmentView::from),
            items: LineView::all(quote),
            totals: TotalsView::of(quote),
        }
    }
}

/// The only fields a PATCH may carry.
const CHANGEABLE: [&str; 4] = ["due_date", "complaint", "cause", "correction"];

/// A repair order as a caller gets it: the JSON, and its version as the `ETag` header.
fn view(order: &RepairOrder) -> ([(HeaderName, String); 1], Json<RepairOrderView>) {
    (
        [(header::ETAG, format!("\"{}\"", order.version()))],
        Json(RepairOrderView::new(order)),
    )
}

/// Refuses a change made from a version that is not the saved one.
fn check_version(expected: Option<i32>, order: &RepairOrder) -> QuoteResult<()> {
    match expected {
        Some(version) if version != order.version() => Err(QuoteError::RepairOrderChangedMeanwhile),
        _ => Ok(()),
    }
}

/// An id that isn't a UUID can't be a repair order: the same answer as an id nobody has.
fn order_id(text: &str) -> QuoteResult<Uuid> {
    Uuid::parse_str(text).map_err(|_| QuoteError::UnknownRepairOrder)
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct RepairOrderListQuery {
    /// Only the orders of this customer.
    customer_id: Option<String>,
    /// Only the orders of this branch.
    branch_id: Option<String>,
    /// Only the orders for this machine.
    equipment_id: Option<String>,
    /// "open", "in_progress", "completed" or "cancelled".
    status: Option<String>,
    /// How many orders to return (1 to 200). Default 50.
    limit: Option<u64>,
    /// How many orders to skip (for the next page). Default 0.
    offset: Option<u64>,
}

impl RepairOrderListQuery {
    fn filter(&self) -> Result<RepairOrderFilter, ApiError> {
        let id = |text: &Option<String>| {
            text.as_deref()
                .map(|text| Uuid::parse_str(text).map_err(|_| ApiError::BadQuery))
                .transpose()
        };
        let status = match self.status.as_deref() {
            None => None,
            Some("open") => Some("open"),
            Some("in_progress") => Some("in_progress"),
            Some("completed") => Some("completed"),
            Some("cancelled") => Some("cancelled"),
            Some(_) => return Err(ApiError::BadQuery),
        };
        Ok(RepairOrderFilter {
            customer_id: id(&self.customer_id)?,
            branch_id: id(&self.branch_id)?,
            equipment_id: id(&self.equipment_id)?,
            status,
        })
    }
}

/// Open a repair order from an **accepted** quote. It starts "open", gets the next number of the
/// year, and its due date is the quote's promise date (if any). A quote has at most one repair
/// order. The body is required (`{}` is fine): send `meter_reading` unless the machine on the quote has one.
#[utoipa::path(post, path = "/quotes/{id}/repair-order", operation_id = "open_repair_order", tag = "repair orders", request_body = OpenRepairOrderInput,
    params(("id" = String, Path, description = "The accepted quote's id.")),
    responses(
        (status = 201, description = "The repair order as saved, with its id and number.", body = RepairOrderView, headers(("ETag" = String, description = "The order's version, like \"0\". Send it back as If-Match to change the order only if it has not moved on."))),
        (status = 400, description = "The body is wrong, or the meter reading is missing (METER_READING_REQUIRED) or not valid.", body = ErrorBody),
        (status = 404, description = "No quote has that id.", body = ErrorBody),
        (status = 409, description = "The quote is not accepted (QUOTE_NOT_ACCEPTED), or already has a repair order (REPAIR_ORDER_EXISTS).", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available, or a saved record is damaged.", body = ErrorBody),
    ))]
pub(super) async fn open(
    State(state): State<AppState>,
    Path(id): Path<String>,
    input: Result<Json<OpenRepairOrderInput>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let quote_id = Uuid::parse_str(&id).map_err(|_| QuoteError::UnknownQuote)?;
    let input = body(input)?;
    let new = NewRepairOrder {
        created_at: now(),
        quote_id,
        meter_reading: input.meter_reading,
    };
    let saved = create_repair_order(&state.db, new).await?;
    Ok((StatusCode::CREATED, view(&saved)))
}

/// List a page of repair orders, newest first, each with its quote's lines and totals.
#[utoipa::path(get, path = "/repair-orders", operation_id = "list_repair_orders", tag = "repair orders", params(RepairOrderListQuery),
    responses(
        (status = 200, description = "A page of repair orders.", body = Vec<RepairOrderView>),
        (status = 400, description = "The query is not valid.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available, or a saved record is damaged.", body = ErrorBody),
    ))]
pub(super) async fn list(
    State(state): State<AppState>,
    query: Result<Query<RepairOrderListQuery>, QueryRejection>,
) -> Result<Json<Vec<RepairOrderView>>, ApiError> {
    let Query(query) = query.map_err(|_| ApiError::BadQuery)?;
    let orders = list_repair_orders(
        &state.db,
        &query.filter()?,
        page_size(query.limit),
        query.offset.unwrap_or(0),
    )
    .await?;
    Ok(Json(orders.iter().map(RepairOrderView::new).collect()))
}

/// Read one repair order.
#[utoipa::path(get, path = "/repair-orders/{id}", operation_id = "read_repair_order", tag = "repair orders",
    params(("id" = String, Path, description = "The repair order's id.")),
    responses(
        (status = 200, description = "The repair order.", body = RepairOrderView, headers(("ETag" = String, description = "The order's version, like \"3\". Send it back as If-Match to change the order only if it has not moved on."))),
        (status = 404, description = "No repair order has that id.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available, or the saved record is damaged.", body = ErrorBody),
    ))]
pub(super) async fn read(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(view(&get_repair_order(&state.db, order_id(&id)?).await?))
}

/// Change the due date and/or write the three paragraphs. Send any of `due_date`, `complaint`,
/// `cause` and `correction` (at least one); what is left out stays, and `null` clears it. The
/// paragraphs are optional free text (at most 4000 characters each, spaces around them are
/// dropped, blank clears). Everything sent is checked first: if one part is refused, nothing
/// changes. A completed or cancelled order can't change.
#[utoipa::path(patch, path = "/repair-orders/{id}", operation_id = "change_repair_order", tag = "repair orders", request_body = RepairOrderChanges,
    params(("id" = String, Path, description = "The repair order's id."), ("If-Match" = Option<String>, Header, description = "The version you read (the ETag). If the order has changed since, nothing is done and the answer is 409 REPAIR_ORDER_CHANGED_MEANWHILE. Leave out to skip the check."),),
    responses(
        (status = 200, description = "The repair order as saved.", body = RepairOrderView, headers(("ETag" = String, description = "The order's version, like \"3\". Send it back as If-Match to change the order only if it has not moved on."))),
        (status = 400, description = "The body is empty or wrong, the If-Match header is not a version, the date is before the day the order was opened, or a paragraph is too long or holds control characters.", body = ErrorBody),
        (status = 404, description = "No repair order has that id.", body = ErrorBody),
        (status = 409, description = "The order is completed or cancelled, or someone changed it meanwhile.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn change(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    input: Result<Json<Map<String, Value>>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let id = order_id(&id)?;
    let expected = if_match(&headers)?;
    let fields = body(input)?;
    // Only these four may be sent, each a string or null, and at least one. Checked before the
    // database is touched.
    if fields.is_empty()
        || fields.iter().any(|(key, value)| {
            !CHANGEABLE.contains(&key.as_str()) || !(value.is_string() || value.is_null())
        })
    {
        return Err(ApiError::BadBody);
    }
    let date = match fields.get("due_date") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(Value::String(text)) => Some(Some(
            text.parse::<NaiveDate>().map_err(|_| ApiError::BadBody)?,
        )),
        Some(_) => return Err(ApiError::BadBody),
    };
    let paragraph = |key: &str| fields.get(key).map(Value::as_str);
    let mut order = get_repair_order(&state.db, id).await?;
    check_version(expected, &order)?;
    if let Some(date) = date {
        order.set_due_date(date)?;
    }
    if let Some(text) = paragraph("complaint") {
        order.set_complaint(text)?;
    }
    if let Some(text) = paragraph("cause") {
        order.set_cause(text)?;
    }
    if let Some(text) = paragraph("correction") {
        order.set_correction(text)?;
    }
    Ok(view(&save_repair_order(&state.db, &order).await?))
}

/// The work begins. Only an open order can start.
#[utoipa::path(post, path = "/repair-orders/{id}/start", operation_id = "start_repair_order", tag = "repair orders",
    params(("id" = String, Path, description = "The repair order's id."), ("If-Match" = Option<String>, Header, description = "The version you read (the ETag). If the order has changed since, nothing is done and the answer is 409 REPAIR_ORDER_CHANGED_MEANWHILE. Leave out to skip the check."),),
    responses(
        (status = 200, description = "The order, now in progress.", body = RepairOrderView, headers(("ETag" = String, description = "The order's version, like \"3\". Send it back as If-Match to change the order only if it has not moved on."))),
        (status = 400, description = "The If-Match header is not a version.", body = ErrorBody),
        (status = 404, description = "No repair order has that id.", body = ErrorBody),
        (status = 409, description = "The order is not open (REPAIR_ORDER_NOT_OPEN), or someone changed it meanwhile.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn start(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let id = order_id(&id)?;
    let expected = if_match(&headers)?;
    let mut order = get_repair_order(&state.db, id).await?;
    check_version(expected, &order)?;
    order.start(now())?;
    Ok(view(&save_repair_order(&state.db, &order).await?))
}

/// The work is done. Only an order that is in progress can be completed.
#[utoipa::path(post, path = "/repair-orders/{id}/complete", operation_id = "complete_repair_order", tag = "repair orders",
    params(("id" = String, Path, description = "The repair order's id."), ("If-Match" = Option<String>, Header, description = "The version you read (the ETag). If the order has changed since, nothing is done and the answer is 409 REPAIR_ORDER_CHANGED_MEANWHILE. Leave out to skip the check."),),
    responses(
        (status = 200, description = "The completed order.", body = RepairOrderView, headers(("ETag" = String, description = "The order's version, like \"3\". Send it back as If-Match to change the order only if it has not moved on."))),
        (status = 400, description = "The If-Match header is not a version.", body = ErrorBody),
        (status = 404, description = "No repair order has that id.", body = ErrorBody),
        (status = 409, description = "The order is not in progress (REPAIR_ORDER_NOT_IN_PROGRESS), or someone changed it meanwhile.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn complete(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let id = order_id(&id)?;
    let expected = if_match(&headers)?;
    let mut order = get_repair_order(&state.db, id).await?;
    check_version(expected, &order)?;
    order.complete(now())?;
    Ok(view(&save_repair_order(&state.db, &order).await?))
}

/// The work will not be done, and why. An open or an in-progress order can be cancelled; a
/// completed or cancelled one can't.
#[utoipa::path(post, path = "/repair-orders/{id}/cancel", operation_id = "cancel_repair_order", tag = "repair orders", request_body = CancelRepairOrderInput,
    params(("id" = String, Path, description = "The repair order's id."), ("If-Match" = Option<String>, Header, description = "The version you read (the ETag). If the order has changed since, nothing is done and the answer is 409 REPAIR_ORDER_CHANGED_MEANWHILE. Leave out to skip the check."),),
    responses(
        (status = 200, description = "The cancelled order.", body = RepairOrderView, headers(("ETag" = String, description = "The order's version, like \"3\". Send it back as If-Match to change the order only if it has not moved on."))),
        (status = 400, description = "The body is wrong, the If-Match header is not a version, or the reason is blank or over 500 characters.", body = ErrorBody),
        (status = 404, description = "No repair order has that id.", body = ErrorBody),
        (status = 409, description = "The order is already completed or cancelled (REPAIR_ORDER_CLOSED), or someone changed it meanwhile.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn cancel(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    input: Result<Json<CancelRepairOrderInput>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let id = order_id(&id)?;
    let expected = if_match(&headers)?;
    let reason = body(input)?.reason;
    let mut order = get_repair_order(&state.db, id).await?;
    check_version(expected, &order)?;
    order.cancel(&reason, now())?;
    Ok(view(&save_repair_order(&state.db, &order).await?))
}
