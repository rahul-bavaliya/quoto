// src/api/customers.rs: the customer calls
//
//   GET   /customers?status=active   list (all of them without ?status)
//   GET   /customers/{id}            read one
//   POST  /customers                 create (a new customer is always active)
//   PUT   /customers/{id}            replace ALL the details (the status is not touched)
//   PATCH /customers/{id}            change only the details that are sent (the rest stays as it is)
//   PATCH /customers/{id}/status     {"status": "inactive"} or {"status": "active"}
//
// There is no DELETE: a customer is never deleted, only made inactive.
// The JSON is only translated here; every rule (phone, email, postal code, province, business
// name...) is the library's, the same as for any other caller. A PATCH replaces a whole
// address when it sends one (`mailing_address`, `shipping_address`); it does not merge inside it.

use super::AppState;
use super::error::{ApiError, ErrorBody};
use super::request::{StatusInput, body, changes, merge};
use crate::common::{Address, Contact, Email, Language, PhoneNumber, PostalCode};
use crate::customer::{
    Customer, create_customer, get_customer, list_customers, set_customer_status, update_customer,
};
use crate::customer_address::CustomerAddress;
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

/// One address, as sent and as returned.
#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(example = json!({
    "address_line1": "1 Main St", "address_line2": null, "city": "Regina",
    "province_code": "SK", "postal_code": "S4P 3Y2"
}))]
pub(super) struct AddressBody {
    address_line1: String,
    address_line2: Option<String>,
    city: String,
    /// The two-letter code, for example "SK" or "AB".
    province_code: String,
    postal_code: String,
}

impl AddressBody {
    fn into_address(self) -> AppResult<Address> {
        Address::new(
            self.address_line1,
            self.address_line2,
            self.city,
            PostalCode::parse(&self.postal_code)?,
            ProvinceCountry::from_province_code(&self.province_code)?,
        )
    }
}

impl From<&Address> for AddressBody {
    fn from(address: &Address) -> Self {
        AddressBody {
            address_line1: address.line1().to_string(),
            address_line2: address.line2().map(str::to_string),
            city: address.city().to_string(),
            province_code: address.province_country().province_code().to_string(),
            postal_code: address.postal_code().to_string(),
        }
    }
}

/// What a caller sends to create or replace a customer. Unknown fields are refused (so a caller
/// can't slip in a `status` or an `id` and think it worked).
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(example = json!({
    "customer_name": "Alex Sample", "business_name": null,
    "mailing_address": {"address_line1": "1 Main St", "address_line2": null, "city": "Regina",
        "province_code": "SK", "postal_code": "S4P 3Y2"},
    "use_shipping_as_mailing": true, "shipping_address": null,
    "phone1": "306-555-0101", "phone2": null, "email": "alex@example.com",
    "language": "en", "gst_registration_no": null
}))]
pub(super) struct CustomerInput {
    customer_name: String,
    /// Give a business name and the customer is a business; leave it out for a person.
    business_name: Option<String>,
    mailing_address: AddressBody,
    /// When true the shipping address is the mailing address and `shipping_address` is not needed.
    use_shipping_as_mailing: bool,
    shipping_address: Option<AddressBody>,
    /// Any usual way of writing a 10-digit phone number.
    phone1: String,
    phone2: Option<String>,
    email: String,
    /// "en" or "fr".
    language: Option<String>,
    gst_registration_no: Option<String>,
}

impl CustomerInput {
    /// Builds a `Customer` with the library's checked types; any rule that fails is returned.
    fn into_customer(self) -> AppResult<Customer> {
        let mailing = self.mailing_address.into_address()?;
        let shipping = self
            .shipping_address
            .map(AddressBody::into_address)
            .transpose()?;
        let customer_address =
            CustomerAddress::new(mailing, self.use_shipping_as_mailing, shipping)?;
        let contact = Contact::new(
            PhoneNumber::parse(&self.phone1)?,
            self.phone2.as_deref().map(PhoneNumber::parse).transpose()?,
            Email::parse(&self.email)?,
            self.language.as_deref().map(Language::parse).transpose()?,
        )?;
        let customer = Customer::new(self.customer_name, customer_address, contact)?
            .with_gst_registration_no(self.gst_registration_no);
        match self.business_name {
            Some(name) => customer.as_business(name),
            None => Ok(customer),
        }
    }
}

/// What a caller gets back. Everything but the id, the status and the two times can be sent back
/// in a PUT, which is also how a PATCH is checked.
#[derive(Serialize, ToSchema)]
pub(super) struct CustomerView {
    /// Given by the database when the customer is created.
    id: String,
    customer_name: String,
    business_name: Option<String>,
    mailing_address: AddressBody,
    use_shipping_as_mailing: bool,
    shipping_address: Option<AddressBody>,
    phone1: String,
    phone2: Option<String>,
    email: String,
    language: Option<String>,
    gst_registration_no: Option<String>,
    /// "active" or "inactive".
    status: &'static str,
    created_at: Option<String>,
    updated_at: Option<String>,
}

impl From<&Customer> for CustomerView {
    fn from(customer: &Customer) -> Self {
        let address = customer.customer_address();
        CustomerView {
            id: customer.id().map(|id| id.to_string()).unwrap_or_default(),
            customer_name: customer.customer_name().to_string(),
            business_name: customer.business_name().map(str::to_string),
            mailing_address: AddressBody::from(address.mailing_address()),
            use_shipping_as_mailing: address.use_shipping_as_mailing(),
            shipping_address: address.explicit_shipping_address().map(AddressBody::from),
            phone1: customer.phone1().to_string(),
            phone2: customer.phone2().map(ToString::to_string),
            email: customer.email().to_string(),
            language: customer.contact().language().map(|l| l.code().to_string()),
            gst_registration_no: customer.gst_registration_no().map(str::to_string),
            status: customer.status().label(),
            created_at: customer.created_at().map(|t| t.to_rfc3339()),
            updated_at: customer.updated_at().map(|t| t.to_rfc3339()),
        }
    }
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(super) struct ListQuery {
    /// Only the customers with this status: "active" or "inactive". Leave out for all.
    status: Option<String>,
}

/// Only for the docs page: any of the fields of a customer. (The call itself reads the body as a
/// plain object and checks it with `CustomerInput`.)
#[derive(ToSchema)]
#[schema(example = json!({"phone1": "306-555-0199", "business_name": null}))]
#[allow(dead_code)]
pub(super) struct CustomerChanges {
    customer_name: Option<String>,
    business_name: Option<String>,
    mailing_address: Option<AddressBody>,
    use_shipping_as_mailing: Option<bool>,
    shipping_address: Option<AddressBody>,
    phone1: Option<String>,
    phone2: Option<String>,
    email: Option<String>,
    language: Option<String>,
    gst_registration_no: Option<String>,
}

/// An id that isn't a UUID can't be a customer: the same answer as an id nobody has.
fn customer_id(text: &str) -> AppResult<Uuid> {
    Uuid::parse_str(text).map_err(|_| AppError::UnknownCustomer)
}

/// List the customers, ordered by name.
#[utoipa::path(get, path = "/customers", operation_id = "list_customers", tag = "customers", params(ListQuery),
    responses(
        (status = 200, description = "The customers.", body = Vec<CustomerView>),
        (status = 400, description = "The status is not 'active' or 'inactive'.", body = ErrorBody),
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
    let customers = list_customers(&state.db, status).await?;
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
        (status = 201, description = "The customer as saved, with its id.", body = CustomerView),
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

/// Replace ALL of a customer's details (every required field must be sent). Its status is not
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
/// field (`business_name`, `phone2`, `language`...). An address is replaced as a whole. The status is
/// not changed here.
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
        (status = 200, description = "The customer with its new status.", body = CustomerView),
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
