// src/api/customers.rs: the customer calls
//
//   GET   /customers?status=active&limit=50&offset=0   a page of customers, ordered by name
//   GET   /customers/{id}                              read one
//   POST  /customers                                   create (a new customer is always active)
//   PUT   /customers/{id}                              replace ALL the details (the status is not touched)
//   PATCH /customers/{id}                              change only the details that are sent
//   PATCH /customers/{id}/status                       {"status": "inactive"} or {"status": "active"}
//
// There is no DELETE: a customer is never deleted, only made inactive.
// These are PEOPLE'S DETAILS. The calls are logged like any call (method, route pattern, status,
// time, error code) and nothing from a customer ever reaches a log: see `claude/logging-rules.md`.
// The JSON is only translated here; every rule is the library's.

use super::AppState;
use super::error::{ApiError, ErrorBody};
use super::request::{StatusInput, body, changes, merge};
use crate::customer::{
    Address, Contact, Customer, Email, PhoneNumber, PostalCode, create_customer, get_customer,
    list_customers, set_customer_status, update_customer,
};
use crate::errors::{QuoteError, QuoteResult};
use crate::province::Province;
use crate::status::Status;
use axum::Json;
use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// A page is this long unless the caller asks for fewer or more (at most `MAX_PAGE`).
const DEFAULT_PAGE: u64 = 50;
const MAX_PAGE: u64 = 200;

/// What a caller sends to create or replace a customer. Unknown fields are refused (so a caller
/// can't slip in a `status` or an `id` and think it worked).
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(example = json!({
    "name": "Jane Smith", "company": "Prairie Farms Ltd.", "designation": "Service Manager",
    "phone": "306-555-0142", "alternate_phone": "306-555-0143", "email": "jane@example.com",
    "address_line1": "123 Main St", "address_line2": "Suite 2", "city": "Regina",
    "province": "SK", "postal_code": "S4P 3Y2"
}))]
pub(super) struct CustomerInput {
    name: String,
    company: Option<String>,
    /// The person's job title, e.g. "Service Manager".
    designation: Option<String>,
    /// Any usual way of writing a 10-digit phone number.
    phone: String,
    alternate_phone: Option<String>,
    email: Option<String>,
    address_line1: String,
    address_line2: Option<String>,
    city: String,
    /// The two-letter code. Only "SK" and "AB" are known so far.
    province: String,
    postal_code: String,
}

impl CustomerInput {
    /// Builds a `Customer` with the library's checked types; any rule that fails is returned.
    fn into_customer(self) -> QuoteResult<Customer> {
        let contact = Contact::new(
            PhoneNumber::parse(&self.phone)?,
            self.alternate_phone
                .as_deref()
                .map(PhoneNumber::parse)
                .transpose()?,
            self.email.as_deref().map(Email::parse).transpose()?,
        );
        let address = Address::new(
            self.address_line1,
            self.address_line2,
            self.city,
            PostalCode::parse(&self.postal_code)?,
            Province::from_code(&self.province)?,
        )?;
        Customer::new(self.name, self.company, self.designation, contact, address)
    }
}

/// Only for the docs page: any of the fields of a customer. (The call itself reads the body as a
/// plain object and checks it with `CustomerInput`.)
#[derive(ToSchema)]
#[schema(example = json!({"phone": "306-555-0199", "email": null}))]
#[allow(dead_code)]
pub(super) struct CustomerChanges {
    name: Option<String>,
    company: Option<String>,
    designation: Option<String>,
    phone: Option<String>,
    alternate_phone: Option<String>,
    email: Option<String>,
    address_line1: Option<String>,
    address_line2: Option<String>,
    city: Option<String>,
    province: Option<String>,
    postal_code: Option<String>,
}

/// What a caller gets back.
#[derive(Serialize, ToSchema)]
pub(super) struct CustomerView {
    /// Given by the database when the customer is created.
    id: String,
    name: String,
    company: Option<String>,
    designation: Option<String>,
    phone: String,
    alternate_phone: Option<String>,
    email: Option<String>,
    address_line1: String,
    address_line2: Option<String>,
    city: String,
    province: String,
    country: String,
    postal_code: String,
    /// "active" or "inactive".
    status: &'static str,
}

impl From<&Customer> for CustomerView {
    fn from(customer: &Customer) -> Self {
        let contact = customer.contact();
        let address = customer.address();
        CustomerView {
            id: customer.id().map(|id| id.to_string()).unwrap_or_default(),
            name: customer.name().to_string(),
            company: customer.company().map(str::to_string),
            designation: customer.designation().map(str::to_string),
            phone: contact.phone().to_string(),
            alternate_phone: contact.alternate_phone().map(PhoneNumber::to_string),
            email: contact.email().map(Email::to_string),
            address_line1: address.line1().to_string(),
            address_line2: address.line2().map(str::to_string),
            city: address.city().to_string(),
            province: address.province().code().to_string(),
            country: address.province().country().code().to_string(),
            postal_code: address.postal_code().to_string(),
            status: customer.status().label(),
        }
    }
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct ListQuery {
    /// Only the customers with this status: "active" or "inactive". Leave out for all.
    status: Option<String>,
    /// How many customers to return (1 to 200). Default 50.
    limit: Option<u64>,
    /// How many customers to skip (for the next page). Default 0.
    offset: Option<u64>,
}

/// An id that isn't a UUID can't be a customer: the same answer as an id nobody has.
fn customer_id(text: &str) -> QuoteResult<Uuid> {
    Uuid::parse_str(text).map_err(|_| QuoteError::UnknownCustomer)
}

/// List a page of customers, ordered by name.
#[utoipa::path(get, path = "/customers", operation_id = "list_customers", tag = "customers", params(ListQuery),
    responses(
        (status = 200, description = "A page of customers.", body = Vec<CustomerView>),
        (status = 400, description = "The status is not 'active' or 'inactive', or the query is not valid.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn list(
    State(state): State<AppState>,
    query: Result<Query<ListQuery>, QueryRejection>,
) -> Result<Json<Vec<CustomerView>>, ApiError> {
    let Query(query) = query.map_err(|_| ApiError::BadQuery)?;
    let status = query
        .status
        .as_deref()
        .map(Status::from_label)
        .transpose()?;
    let limit = query.limit.unwrap_or(DEFAULT_PAGE).clamp(1, MAX_PAGE);
    let customers = list_customers(&state.db, status, limit, query.offset.unwrap_or(0)).await?;
    Ok(Json(customers.iter().map(CustomerView::from).collect()))
}

/// Read one customer.
#[utoipa::path(get, path = "/customers/{id}", operation_id = "read_customer", tag = "customers",
    params(("id" = String, Path, description = "The customer id.")),
    responses(
        (status = 200, description = "The customer.", body = CustomerView),
        (status = 404, description = "No customer has that id.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn read(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<CustomerView>, ApiError> {
    let customer = get_customer(&state.db, customer_id(&id)?).await?;
    Ok(Json(CustomerView::from(&customer)))
}

/// Create a customer. A new customer is always active.
#[utoipa::path(post, path = "/customers", operation_id = "create_customer", tag = "customers", request_body = CustomerInput,
    responses(
        (status = 201, description = "The customer as saved, with their id.", body = CustomerView),
        (status = 400, description = "The body is wrong, or a rule refused a value.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn create(
    State(state): State<AppState>,
    input: Result<Json<CustomerInput>, JsonRejection>,
) -> Result<(StatusCode, Json<CustomerView>), ApiError> {
    let customer = body(input)?.into_customer()?;
    let saved = create_customer(&state.db, &customer).await?;
    Ok((StatusCode::CREATED, Json(CustomerView::from(&saved))))
}

/// Replace ALL of a customer's details (every required field must be sent). Their status is not
/// changed. To change only some, use PATCH.
#[utoipa::path(put, path = "/customers/{id}", operation_id = "replace_customer", tag = "customers", request_body = CustomerInput,
    params(("id" = String, Path, description = "The customer id.")),
    responses(
        (status = 200, description = "The customer as saved.", body = CustomerView),
        (status = 400, description = "The body is wrong, or a rule refused a value.", body = ErrorBody),
        (status = 404, description = "No customer has that id.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn replace(
    State(state): State<AppState>,
    Path(id): Path<String>,
    input: Result<Json<CustomerInput>, JsonRejection>,
) -> Result<Json<CustomerView>, ApiError> {
    let id = customer_id(&id)?;
    let new_details = body(input)?.into_customer()?;
    // The details are replaced, the status is not: a closed customer must not reopen by accident.
    let current = get_customer(&state.db, id).await?;
    let updated = update_customer(
        &state.db,
        &new_details.with_id(id).with_status(current.status()),
    )
    .await?;
    Ok(Json(CustomerView::from(&updated)))
}

/// Change only the details that are sent; the rest stays as it is. Send `null` to clear an optional
/// field (`company`, `email`...). The status is not changed here.
#[utoipa::path(patch, path = "/customers/{id}", operation_id = "change_customer", tag = "customers", request_body = CustomerChanges,
    params(("id" = String, Path, description = "The customer id.")),
    responses(
        (status = 200, description = "The customer as saved.", body = CustomerView),
        (status = 400, description = "The body is empty or wrong, or a rule refused a value.", body = ErrorBody),
        (status = 404, description = "No customer has that id.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn change(
    State(state): State<AppState>,
    Path(id): Path<String>,
    input: Result<Json<Map<String, Value>>, JsonRejection>,
) -> Result<Json<CustomerView>, ApiError> {
    let id = customer_id(&id)?;
    let changes = changes(input)?;
    let current = get_customer(&state.db, id).await?;
    let customer =
        merge::<CustomerInput>(&CustomerView::from(&current), changes)?.into_customer()?;
    let updated = update_customer(
        &state.db,
        &customer.with_id(id).with_status(current.status()),
    )
    .await?;
    Ok(Json(CustomerView::from(&updated)))
}

/// Make a customer active or inactive. (A customer is never deleted.)
#[utoipa::path(patch, path = "/customers/{id}/status", operation_id = "change_customer_status", tag = "customers", request_body = StatusInput,
    params(("id" = String, Path, description = "The customer id.")),
    responses(
        (status = 200, description = "The customer with their new status.", body = CustomerView),
        (status = 400, description = "The status is not 'active' or 'inactive'.", body = ErrorBody),
        (status = 404, description = "No customer has that id.", body = ErrorBody),
        (status = 401, description = "No key, or the wrong one.", body = ErrorBody),
        (status = 503, description = "The database is not available.", body = ErrorBody),
    ))]
pub(super) async fn change_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    input: Result<Json<StatusInput>, JsonRejection>,
) -> Result<Json<CustomerView>, ApiError> {
    let id = customer_id(&id)?;
    let status = Status::from_label(&body(input)?.status)?;
    let customer = set_customer_status(&state.db, id, status).await?;
    Ok(Json(CustomerView::from(&customer)))
}
