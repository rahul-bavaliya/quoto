// src/api/quotes.rs: the quote calls
//
//   GET   /quotes?customer_id=&branch_id=&equipment_id=&limit=&offset=   a page of quotes, newest first
//   GET   /quotes/{id}             read one (with its lines and totals, its version, and an ETag)
//   GET   /quotes/{id}/pdf         the quote as a PDF file to print or send
//   POST  /quotes                  create (always pending; the number is handed out here)
//   PATCH /quotes/{id}             change the expiry date and/or the promise date
//   POST  /quotes/{id}/lines       add one line (lines are only ever added)
//   PATCH /quotes/{id}/customer    {"customer_id": "<id>"}   add the customer (once)
//   PATCH /quotes/{id}/equipment   {"equipment_id": "<id>"}  add the machine (once; the customer first)
//   POST  /quotes/{id}/accept      the customer says yes
//   POST  /quotes/{id}/reject      {"reason": "..."}         the customer says no, and why
//   POST  /quotes/{id}/reopen      a rejected quote goes back to pending
//
// Every quote has a `version` (0 when made, +1 with every change), sent in the body and as the
// `ETag`. A call that changes a quote may send it back as `If-Match`: if the quote has changed
// since, nothing is done and the answer is `409 QUOTE_CHANGED_MEANWHILE` (read it again). Without
// the header a change is still checked at the moment it is saved, but not against an old screen.
//
// A quote is never erased and has no DELETE and no status call: its status moves only through
// accept / reject / reopen, so a decision can't be made by accident. The branch, customer and
// machine on a quote are copies of how they were when they were added: renaming a customer later
// does not change an old quote. Nothing from a quote reaches a log (see `claude/logging-rules.md`).
// The JSON is only translated here; every rule is the library's.

use super::AppState;
use super::branches::BranchView;
use super::customers::CustomerView;
use super::equipment::EquipmentView;
use super::error::{ApiError, ErrorBody};
use super::request::{body, changes, page_size};
use crate::errors::{QuoteError, QuoteResult};
use crate::quote::{
    LineItem, LineKind, NewQuote, Quote, QuoteFilter, QuoteStatus, create_quote, get_quote,
    list_quotes, save_quote,
};
use crate::{get_customer, get_equipment};
use axum::Json;
use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderName, StatusCode, header};
use axum::response::IntoResponse;
use chrono::{Local, NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// One line a caller sends.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(example = json!({
    "description": "Oil change labour", "quantity": 1.5, "unit_price_cents": 12000, "kind": "labour"
}))]
pub(super) struct LineInput {
    /// What it is for (at most 200 characters).
    description: String,
    /// More than zero, at most 1,000,000.
    quantity: f64,
    /// The price of one, in cents; never negative, at most 100,000,000 (a credit is also sent
    /// as a positive price: the kind makes it a credit).
    unit_price_cents: i64,
    /// "labour", "part", "fee" or "credit".
    kind: String,
}

impl LineInput {
    fn into_line(self) -> QuoteResult<LineItem> {
        let kind = LineKind::from_stored(&self.kind).ok_or(QuoteError::InvalidLineKind)?;
        LineItem::new(self.description, self.quantity, self.unit_price_cents, kind)
    }
}

/// What a caller sends to create a quote. Only the branch is needed (a quick quote); the
/// customer, the machine and the lines can come now or later.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(example = json!({
    "branch_id": "00000000-0000-0000-0000-000000000000",
    "customer_id": null, "equipment_id": null,
    "items": [{"description": "Oil change labour", "quantity": 1.5, "unit_price_cents": 12000, "kind": "labour"}]
}))]
pub(super) struct NewQuoteInput {
    /// The branch the quote is made at (it decides the taxes).
    branch_id: String,
    /// The customer, if known. Leave out or null to add later.
    customer_id: Option<String>,
    /// The machine, if known. It needs the customer, and must be theirs.
    equipment_id: Option<String>,
    /// The lines, in order. Leave out for none.
    items: Option<Vec<LineInput>>,
}

/// What a caller can change on a saved quote: either date, or both. Dates are `YYYY-MM-DD`.
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(example = json!({"expires_on": "2026-12-31", "promise_date": "2027-01-15"}))]
pub(super) struct QuoteChanges {
    /// The last day the prices are honoured; not before the quote's own date.
    #[schema(value_type = Option<String>, format = Date)]
    expires_on: Option<NaiveDate>,
    /// The day the machine will be ready; not before the quote's own date.
    #[schema(value_type = Option<String>, format = Date)]
    promise_date: Option<NaiveDate>,
}

/// The body of "add the customer".
#[derive(ToSchema)]
#[schema(example = json!({"customer_id": "00000000-0000-0000-0000-000000000000"}))]
#[allow(dead_code)]
pub(super) struct QuoteCustomerInput {
    /// The customer's id. The key must be there and must be an id (a customer is never removed).
    customer_id: String,
}

/// The body of "add the machine".
#[derive(ToSchema)]
#[schema(example = json!({"equipment_id": "00000000-0000-0000-0000-000000000000"}))]
#[allow(dead_code)]
pub(super) struct QuoteEquipmentInput {
    /// The machine's id. The key must be there and must be an id.
    equipment_id: String,
}

/// The body of "reject".
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(example = json!({"reason": "Too expensive"}))]
pub(super) struct RejectInput {
    /// Why (not blank, at most 500 characters). It is the customer's own words: never logged.
    reason: String,
}

/// One line of a saved quote.
#[derive(Serialize, ToSchema)]
pub(super) struct LineView {
    id: String,
    /// "labour", "part", "fee" or "credit".
    kind: String,
    description: String,
    quantity: f64,
    /// The price of one, as entered (never negative).
    unit_price_cents: i64,
    /// Quantity times price; **negative for a credit**.
    line_total_cents: i64,
}

/// What the quote adds up to, in cents. The taxes follow the branch's province.
#[derive(Serialize, ToSchema)]
pub(super) struct TotalsView {
    labour_cents: i64,
    parts_cents: i64,
    fees_cents: i64,
    /// Zero or below.
    credit_cents: i64,
    subtotal_cents: i64,
    gst_cents: i64,
    pst_cents: i64,
    total_cents: i64,
}

/// What a caller gets back.
#[derive(Serialize, ToSchema)]
pub(super) struct QuoteView {
    id: String,
    /// 0 when the quote is made, +1 with every change. Also sent as the `ETag` header: send it
    /// back as `If-Match` on a change to refuse the change if the quote moved on since you read it.
    version: i32,
    /// `Q-<year>-<8 digits>`, handed out when the quote was created.
    number: String,
    /// "pending", "accepted" or "rejected".
    status: &'static str,
    /// Only on a rejected quote.
    rejection_reason: Option<String>,
    /// Pending and past its expiry date. (Worked out when asked; never stored.)
    expired: bool,
    /// The shop's local time, `YYYY-MM-DDTHH:MM:SS`.
    created_at: String,
    /// `YYYY-MM-DD`.
    expires_on: String,
    promise_date: Option<String>,
    /// The branch, the customer and the machine **as they were when they were added**.
    branch: BranchView,
    customer: Option<CustomerView>,
    equipment: Option<EquipmentView>,
    items: Vec<LineView>,
    totals: TotalsView,
}

impl QuoteView {
    fn new(quote: &Quote, today: NaiveDate) -> Self {
        let rejection_reason = match quote.status() {
            QuoteStatus::Rejected { reason } => Some(reason.clone()),
            _ => None,
        };
        QuoteView {
            id: quote.id().map(|id| id.to_string()).unwrap_or_default(),
            version: quote.version(),
            number: quote.number().to_string(),
            status: quote.status().label(),
            rejection_reason,
            expired: quote.is_expired(today),
            created_at: quote.created_at().format("%Y-%m-%dT%H:%M:%S").to_string(),
            expires_on: quote.expires_on().to_string(),
            promise_date: quote.promise_date().map(|date| date.to_string()),
            branch: BranchView::from(quote.branch()),
            customer: quote.customer().map(CustomerView::from),
            equipment: quote.equipment().map(EquipmentView::from),
            items: quote
                .items()
                .iter()
                .map(|line| LineView {
                    id: line.id().map(|id| id.to_string()).unwrap_or_default(),
                    kind: line.kind().label().to_lowercase(),
                    description: line.description().to_string(),
                    quantity: line.quantity(),
                    unit_price_cents: line.unit_price_cents(),
                    line_total_cents: line.line_total(),
                })
                .collect(),
            totals: TotalsView {
                labour_cents: quote.labour_total(),
                parts_cents: quote.parts_total(),
                fees_cents: quote.fees_total(),
                credit_cents: quote.credit_total(),
                subtotal_cents: quote.subtotal(),
                gst_cents: quote.gst_amount(),
                pst_cents: quote.pst_amount(),
                total_cents: quote.total(),
            },
        }
    }
}

/// The shop's clock. A quote never looks at it: the API hands the time in.
fn now() -> NaiveDateTime {
    Local::now().naive_local()
}

/// A quote as a caller gets it: the JSON, and its version as the `ETag` header.
fn view(quote: &Quote) -> ([(HeaderName, String); 1], Json<QuoteView>) {
    (
        [(header::ETAG, format!("\"{}\"", quote.version()))],
        Json(QuoteView::new(quote, now().date())),
    )
}

/// The version a caller says it read: `If-Match: "3"` (or `3`). `*` and no header mean "any".
/// Anything else is refused before the database is touched.
fn if_match(headers: &HeaderMap) -> Result<Option<i32>, ApiError> {
    let Some(value) = headers.get(header::IF_MATCH) else {
        return Ok(None);
    };
    let text = value.to_str().map_err(|_| ApiError::BadIfMatch)?.trim();
    if text == "*" {
        return Ok(None);
    }
    text.trim_matches('"')
        .parse::<i32>()
        .ok()
        .filter(|version| *version >= 0)
        .map(Some)
        .ok_or(ApiError::BadIfMatch)
}

/// Refuses a change made from a version that is not the saved one.
fn check_version(expected: Option<i32>, quote: &Quote) -> QuoteResult<()> {
    match expected {
        Some(version) if version != quote.version() => Err(QuoteError::QuoteChangedMeanwhile),
        _ => Ok(()),
    }
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct ListQuery {
    /// Only the quotes of this customer.
    customer_id: Option<String>,
    /// Only the quotes made at this branch.
    branch_id: Option<String>,
    /// Only the quotes for this machine.
    equipment_id: Option<String>,
    /// How many quotes to return (1 to 200). Default 50.
    limit: Option<u64>,
    /// How many quotes to skip (for the next page). Default 0.
    offset: Option<u64>,
}

impl ListQuery {
    fn filter(&self) -> Result<QuoteFilter, ApiError> {
        let id = |text: &Option<String>| {
            text.as_deref()
                .map(|text| Uuid::parse_str(text).map_err(|_| ApiError::BadQuery))
                .transpose()
        };
        Ok(QuoteFilter {
            customer_id: id(&self.customer_id)?,
            branch_id: id(&self.branch_id)?,
            equipment_id: id(&self.equipment_id)?,
        })
    }
}

/// An id that isn't a UUID can't be a quote: the same answer as an id nobody has.
fn quote_id(text: &str) -> QuoteResult<Uuid> {
    Uuid::parse_str(text).map_err(|_| QuoteError::UnknownQuote)
}

/// An id in a body that isn't a UUID can't be a branch, customer or machine either.
fn party_id(text: &str, unknown: QuoteError) -> QuoteResult<Uuid> {
    Uuid::parse_str(text).map_err(|_| unknown)
}

/// The body of "add the customer / machine": exactly one key, holding an id (never null: a
/// party is added once and never removed).
fn only_id(
    input: Result<Json<Map<String, Value>>, JsonRejection>,
    key: &str,
    unknown: QuoteError,
) -> Result<Uuid, ApiError> {
    let fields = body(input)?;
    match (fields.len(), fields.get(key)) {
        (1, Some(Value::String(text))) => Ok(party_id(text, unknown)?),
        _ => Err(ApiError::BadBody),
    }
}

/// List a page of quotes, newest first, each with its lines.
#[utoipa::path(get, path = "/quotes", operation_id = "list_quotes", tag = "quotes", params(ListQuery),
    responses(
        (status = 200, description = "A page of quotes.", body = Vec<QuoteView>),
        (status = 400, description = "The query is not valid.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available, or a saved quote is damaged.", body = ErrorBody),
    ))]
pub(super) async fn list(
    State(state): State<AppState>,
    query: Result<Query<ListQuery>, QueryRejection>,
) -> Result<Json<Vec<QuoteView>>, ApiError> {
    let Query(query) = query.map_err(|_| ApiError::BadQuery)?;
    let quotes = list_quotes(
        &state.db,
        &query.filter()?,
        page_size(query.limit),
        query.offset.unwrap_or(0),
    )
    .await?;
    let today = now().date();
    Ok(Json(
        quotes
            .iter()
            .map(|quote| QuoteView::new(quote, today))
            .collect(),
    ))
}

/// Read one quote.
#[utoipa::path(get, path = "/quotes/{id}", operation_id = "read_quote", tag = "quotes",
    params(("id" = String, Path, description = "The quote's id.")),
    responses(
        (status = 200, description = "The quote.", body = QuoteView, headers(("ETag" = String, description = "The quote's version, like \"3\". Send it back as If-Match to change the quote only if it has not moved on."))),
        (status = 404, description = "No quote has that id.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available, or the saved quote is damaged.", body = ErrorBody),
    ))]
pub(super) async fn read(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(view(&get_quote(&state.db, quote_id(&id)?).await?))
}

/// The body of the PDF answer: a file, not JSON.
#[derive(ToSchema)]
#[schema(value_type = String, format = Binary)]
#[allow(dead_code)]
pub(super) struct PdfFile(Vec<u8>);

/// The quote as a PDF file (Letter size, any number of pages): the same lines and totals as the
/// JSON, laid out to print or send to the customer. A browser can't send the key, so open it
/// from the docs page ("Try it out" then "Download file") or with `curl -o quote.pdf`. The answer
/// is never cached, because it holds the customer's details.
#[utoipa::path(get, path = "/quotes/{id}/pdf", operation_id = "print_quote", tag = "quotes",
    params(("id" = String, Path, description = "The quote's id.")),
    responses(
        (status = 200, description = "The quote as a PDF file.", content_type = "application/pdf", body = PdfFile),
        (status = 404, description = "No quote has that id.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available, or the saved quote is damaged.", body = ErrorBody),
    ))]
pub(super) async fn print(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let quote = get_quote(&state.db, quote_id(&id)?).await?;
    let file = quote.to_pdf(now().date());
    // The number is `Q-<year>-<digits>`: safe to put in a file name.
    let name = format!("inline; filename=\"{}.pdf\"", quote.number());
    let headers: [(HeaderName, String); 3] = [
        (header::CONTENT_TYPE, "application/pdf".to_string()),
        (header::CONTENT_DISPOSITION, name),
        (header::CACHE_CONTROL, "no-store".to_string()),
    ];
    Ok((StatusCode::OK, headers, file))
}

/// Create a quote. It is always pending and gets the next number of the year. Only the branch is
/// needed; the customer and the machine (which must be the customer's) and the lines may come now
/// or later. Nothing is saved if anything is refused, and a refused quote does not use up a number.
#[utoipa::path(post, path = "/quotes", operation_id = "create_quote", tag = "quotes", request_body = NewQuoteInput,
    responses(
        (status = 201, description = "The quote as saved, with its id and number.", body = QuoteView, headers(("ETag" = String, description = "The quote's version, like \"3\". Send it back as If-Match to change the quote only if it has not moved on."))),
        (status = 400, description = "The body is wrong, or a rule refused a value.", body = ErrorBody),
        (status = 404, description = "The branch, customer or machine is not saved.", body = ErrorBody),
        (status = 409, description = "A party is inactive, the machine is not the customer's or needs a customer, or a credit is bigger than the subtotal.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn create(
    State(state): State<AppState>,
    input: Result<Json<NewQuoteInput>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let input = body(input)?;
    // Everything that can be checked without the database is checked first.
    let items = input
        .items
        .unwrap_or_default()
        .into_iter()
        .map(LineInput::into_line)
        .collect::<QuoteResult<Vec<_>>>()?;
    let optional = |text: Option<String>, unknown: QuoteError| {
        text.map(|text| party_id(&text, unknown)).transpose()
    };
    let new = NewQuote {
        created_at: now(),
        branch_id: party_id(&input.branch_id, QuoteError::UnknownBranch)?,
        customer_id: optional(input.customer_id, QuoteError::UnknownCustomer)?,
        equipment_id: optional(input.equipment_id, QuoteError::UnknownEquipment)?,
        items,
    };
    let saved = create_quote(&state.db, new).await?;
    Ok((StatusCode::CREATED, view(&saved)))
}

/// Change the dates. Send either or both (`YYYY-MM-DD`); what is left out stays. A date can't be
/// cleared, and neither can be before the quote's own date.
#[utoipa::path(patch, path = "/quotes/{id}", operation_id = "change_quote", tag = "quotes", request_body = QuoteChanges,
    params(("id" = String, Path, description = "The quote's id."), ("If-Match" = Option<String>, Header, description = "The version you read (the ETag). If the quote has changed since, nothing is done and the answer is 409 QUOTE_CHANGED_MEANWHILE. Leave out to skip the check."),),
    responses(
        (status = 200, description = "The quote as saved.", body = QuoteView, headers(("ETag" = String, description = "The quote's version, like \"3\". Send it back as If-Match to change the quote only if it has not moved on."))),
        (status = 400, description = "The body is empty or wrong, or a date is before the quote's own date.", body = ErrorBody),
        (status = 404, description = "No quote has that id.", body = ErrorBody),
        (status = 409, description = "Someone changed the quote meanwhile; read it again.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn change(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    input: Result<Json<Map<String, Value>>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let id = quote_id(&id)?;
    let expected = if_match(&headers)?;
    let fields = changes(input)?;
    // A `null` would mean "clear it", which a date can't do: refused rather than ignored.
    if fields.values().any(|value| !value.is_string()) {
        return Err(ApiError::BadBody);
    }
    let dates: QuoteChanges =
        serde_json::from_value(Value::Object(fields)).map_err(|_| ApiError::BadBody)?;
    let mut quote = get_quote(&state.db, id).await?;
    check_version(expected, &quote)?;
    if let Some(date) = dates.expires_on {
        quote.set_expiry(date)?;
    }
    if let Some(date) = dates.promise_date {
        quote.set_promise_date(date)?;
    }
    Ok(view(&save_quote(&state.db, &quote).await?))
}

/// Add one line at the end. Lines are only ever added, never changed or removed (a mistake on a
/// quote that is still pending is fixed with a credit line, and the old lines stay as they were).
/// Only a pending quote can change.
#[utoipa::path(post, path = "/quotes/{id}/lines", operation_id = "add_quote_line", tag = "quotes", request_body = LineInput,
    params(("id" = String, Path, description = "The quote's id."), ("If-Match" = Option<String>, Header, description = "The version you read (the ETag). If the quote has changed since, nothing is done and the answer is 409 QUOTE_CHANGED_MEANWHILE. Leave out to skip the check."),),
    responses(
        (status = 201, description = "The quote with the new line at the end.", body = QuoteView, headers(("ETag" = String, description = "The quote's version, like \"3\". Send it back as If-Match to change the quote only if it has not moved on."))),
        (status = 400, description = "The body is wrong, or a rule refused a value.", body = ErrorBody),
        (status = 404, description = "No quote has that id.", body = ErrorBody),
        (status = 409, description = "The quote is already decided, is full (200 lines), a credit is bigger than the subtotal, or someone changed it meanwhile.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn add_line(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    input: Result<Json<LineInput>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let id = quote_id(&id)?;
    let expected = if_match(&headers)?;
    let line = body(input)?.into_line()?;
    let mut quote = get_quote(&state.db, id).await?;
    check_version(expected, &quote)?;
    quote.add_item(line)?;
    Ok((
        StatusCode::CREATED,
        view(&save_quote(&state.db, &quote).await?),
    ))
}

/// Add the customer to a quote that has none. A quote has one customer, added once.
#[utoipa::path(patch, path = "/quotes/{id}/customer", operation_id = "add_quote_customer", tag = "quotes", request_body = QuoteCustomerInput,
    params(("id" = String, Path, description = "The quote's id."), ("If-Match" = Option<String>, Header, description = "The version you read (the ETag). If the quote has changed since, nothing is done and the answer is 409 QUOTE_CHANGED_MEANWHILE. Leave out to skip the check."),),
    responses(
        (status = 200, description = "The quote with its customer.", body = QuoteView, headers(("ETag" = String, description = "The quote's version, like \"3\". Send it back as If-Match to change the quote only if it has not moved on."))),
        (status = 400, description = "The body is not exactly {\"customer_id\": id}.", body = ErrorBody),
        (status = 404, description = "No quote has that id, or the customer is not saved.", body = ErrorBody),
        (status = 409, description = "The quote already has a customer or is already decided, the customer is inactive, or someone changed the quote meanwhile.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn add_customer(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    input: Result<Json<Map<String, Value>>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let id = quote_id(&id)?;
    let expected = if_match(&headers)?;
    let customer_id = only_id(input, "customer_id", QuoteError::UnknownCustomer)?;
    let mut quote = get_quote(&state.db, id).await?;
    check_version(expected, &quote)?;
    quote.attach_customer(get_customer(&state.db, customer_id).await?)?;
    Ok(view(&save_quote(&state.db, &quote).await?))
}

/// Add the machine to a quote that has none. The customer comes first, and the machine must be
/// theirs. A quote has one machine, added once.
#[utoipa::path(patch, path = "/quotes/{id}/equipment", operation_id = "add_quote_equipment", tag = "quotes", request_body = QuoteEquipmentInput,
    params(("id" = String, Path, description = "The quote's id."), ("If-Match" = Option<String>, Header, description = "The version you read (the ETag). If the quote has changed since, nothing is done and the answer is 409 QUOTE_CHANGED_MEANWHILE. Leave out to skip the check."),),
    responses(
        (status = 200, description = "The quote with its machine.", body = QuoteView, headers(("ETag" = String, description = "The quote's version, like \"3\". Send it back as If-Match to change the quote only if it has not moved on."))),
        (status = 400, description = "The body is not exactly {\"equipment_id\": id}.", body = ErrorBody),
        (status = 404, description = "No quote has that id, or the machine is not saved.", body = ErrorBody),
        (status = 409, description = "The quote has no customer yet, already has a machine, is already decided, the machine is archived or belongs to someone else, or someone changed the quote meanwhile.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn add_equipment(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    input: Result<Json<Map<String, Value>>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let id = quote_id(&id)?;
    let expected = if_match(&headers)?;
    let equipment_id = only_id(input, "equipment_id", QuoteError::UnknownEquipment)?;
    let mut quote = get_quote(&state.db, id).await?;
    check_version(expected, &quote)?;
    quote.attach_equipment(get_equipment(&state.db, equipment_id).await?)?;
    Ok(view(&save_quote(&state.db, &quote).await?))
}

/// The customer accepts. Needs a customer and a machine, and prices that have not expired
/// (move the expiry date first). An accepted quote is final.
#[utoipa::path(post, path = "/quotes/{id}/accept", operation_id = "accept_quote", tag = "quotes",
    params(("id" = String, Path, description = "The quote's id."), ("If-Match" = Option<String>, Header, description = "The version you read (the ETag). If the quote has changed since, nothing is done and the answer is 409 QUOTE_CHANGED_MEANWHILE. Leave out to skip the check."),),
    responses(
        (status = 400, description = "The If-Match header is not a version.", body = ErrorBody),
        (status = 200, description = "The accepted quote.", body = QuoteView, headers(("ETag" = String, description = "The quote's version, like \"3\". Send it back as If-Match to change the quote only if it has not moved on."))),
        (status = 404, description = "No quote has that id.", body = ErrorBody),
        (status = 409, description = "Already decided, no customer or machine yet, expired, or someone changed it meanwhile.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn accept(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let id = quote_id(&id)?;
    let expected = if_match(&headers)?;
    let mut quote = get_quote(&state.db, id).await?;
    check_version(expected, &quote)?;
    quote.accept(now().date())?;
    Ok(view(&save_quote(&state.db, &quote).await?))
}

/// The customer says no, and why. An expired quote can still be rejected.
#[utoipa::path(post, path = "/quotes/{id}/reject", operation_id = "reject_quote", tag = "quotes", request_body = RejectInput,
    params(("id" = String, Path, description = "The quote's id."), ("If-Match" = Option<String>, Header, description = "The version you read (the ETag). If the quote has changed since, nothing is done and the answer is 409 QUOTE_CHANGED_MEANWHILE. Leave out to skip the check."),),
    responses(
        (status = 200, description = "The rejected quote.", body = QuoteView, headers(("ETag" = String, description = "The quote's version, like \"3\". Send it back as If-Match to change the quote only if it has not moved on."))),
        (status = 400, description = "The body is wrong, or the reason is blank or over 500 characters.", body = ErrorBody),
        (status = 404, description = "No quote has that id.", body = ErrorBody),
        (status = 409, description = "Already decided, or someone changed it meanwhile.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn reject(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    input: Result<Json<RejectInput>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let id = quote_id(&id)?;
    let expected = if_match(&headers)?;
    let reason = body(input)?.reason;
    let mut quote = get_quote(&state.db, id).await?;
    check_version(expected, &quote)?;
    quote.reject(&reason)?;
    Ok(view(&save_quote(&state.db, &quote).await?))
}

/// A rejected quote goes back to pending (the customer came back); the old reason is dropped.
/// An accepted quote can't be reopened.
#[utoipa::path(post, path = "/quotes/{id}/reopen", operation_id = "reopen_quote", tag = "quotes",
    params(("id" = String, Path, description = "The quote's id."), ("If-Match" = Option<String>, Header, description = "The version you read (the ETag). If the quote has changed since, nothing is done and the answer is 409 QUOTE_CHANGED_MEANWHILE. Leave out to skip the check."),),
    responses(
        (status = 400, description = "The If-Match header is not a version.", body = ErrorBody),
        (status = 200, description = "The pending quote.", body = QuoteView, headers(("ETag" = String, description = "The quote's version, like \"3\". Send it back as If-Match to change the quote only if it has not moved on."))),
        (status = 404, description = "No quote has that id.", body = ErrorBody),
        (status = 409, description = "The quote is not rejected, or someone changed it meanwhile.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn reopen(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let id = quote_id(&id)?;
    let expected = if_match(&headers)?;
    let mut quote = get_quote(&state.db, id).await?;
    check_version(expected, &quote)?;
    quote.reopen()?;
    Ok(view(&save_quote(&state.db, &quote).await?))
}
