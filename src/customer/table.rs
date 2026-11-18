// src/customer/table.rs: the `customers` table in the database, and its CRUD
//
// Create, read and update are here. **There is no delete, on purpose**: a customer is never
// deleted (old quotes point at them). When they are finished with, `set_customer_status` makes
// them `Inactive`. The table is built by a migration; the struct in `row` says how it is read; the rules
// (a name is not blank, a phone / email / postal code are real, the province is known, the
// status is a known word) are checked in Rust when a `Customer` is built and again when a row
// is read.
//
// This table holds PEOPLE'S DETAILS. Nothing here logs a value from it (see
// `claude/logging-rules.md`), and while Quoto has no login (Phase 9) only made-up customers
// may be saved.

use super::{Address, Contact, Customer, Email, PhoneNumber, PostalCode};
use crate::errors::{QuoteError, QuoteResult};
use crate::province::Province;
use crate::status::Status;
use sea_orm::sea_query::Expr;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait};
use sea_orm::{QueryFilter, QueryOrder, QuerySelect, Set};
use tracing::debug;
use uuid::Uuid;

pub(crate) use row::Entity as CustomerTable;
use row::{ActiveModel, Column, Entity};

/// The `customers` table as a struct.
mod row {
    use sea_orm::entity::prelude::*;

    #[sea_orm::model]
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize)]
    #[sea_orm(table_name = "customers")]
    pub struct Model {
        /// Made by the database, never by Quoto.
        #[sea_orm(
            primary_key,
            auto_increment = false,
            default_expr = "Expr::cust(\"gen_random_uuid()\")"
        )]
        pub id: Uuid,
        #[sea_orm(column_type = "Text")]
        pub name: String,
        #[sea_orm(column_type = "Text")]
        pub company: Option<String>,
        #[sea_orm(column_type = "Text")]
        pub designation: Option<String>,
        #[sea_orm(column_type = "Text")]
        pub phone: String,
        #[sea_orm(column_type = "Text")]
        pub alternate_phone: Option<String>,
        #[sea_orm(column_type = "Text")]
        pub email: Option<String>,
        #[sea_orm(column_type = "Text")]
        pub address_line1: String,
        #[sea_orm(column_type = "Text")]
        pub address_line2: Option<String>,
        #[sea_orm(column_type = "Text")]
        pub city: String,
        /// "SK", "AB"... (the province's code; its taxes live in the `provinces` table)
        #[sea_orm(column_type = "Text")]
        pub province_code: String,
        /// Written from the province ("CA"), never read back: the country follows from the province.
        #[sea_orm(column_type = "Text")]
        pub country_code: String,
        #[sea_orm(column_type = "Text")]
        pub postal_code: String,
        /// "active" or "inactive" (see `Status`).
        #[sea_orm(column_type = "Text")]
        pub status: String,
        #[sea_orm(default_expr = "Expr::current_timestamp()")]
        pub created_at: DateTimeUtc,
        #[sea_orm(default_expr = "Expr::current_timestamp()")]
        pub updated_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// A failed read or write: the cause goes to the debug log, the person sees a short message.
fn database_error(cause: sea_orm::DbErr) -> QuoteError {
    debug!(%cause, "customer query failed");
    QuoteError::DatabaseUnavailable
}

/// Rebuilds a `Customer` from a row, re-checking everything (a damaged row is refused).
fn customer_from_row(row: row::Model) -> QuoteResult<Customer> {
    let contact = Contact::new(
        PhoneNumber::parse(&row.phone)?,
        row.alternate_phone
            .as_deref()
            .map(PhoneNumber::parse)
            .transpose()?,
        row.email.as_deref().map(Email::parse).transpose()?,
    );
    let address = Address::new(
        row.address_line1,
        row.address_line2,
        row.city,
        PostalCode::parse(&row.postal_code)?,
        Province::from_code(&row.province_code)?,
    )?;
    Ok(
        Customer::new(row.name, row.company, row.designation, contact, address)?
            .with_id(row.id)
            .with_status(Status::from_label(&row.status)?),
    )
}

/// The row values of a customer (everything except the id and the two timestamps).
fn row_values(customer: &Customer) -> ActiveModel {
    let contact = customer.contact();
    let address = customer.address();
    ActiveModel {
        name: Set(customer.name().to_string()),
        company: Set(customer.company().map(str::to_string)),
        designation: Set(customer.designation().map(str::to_string)),
        phone: Set(contact.phone().to_string()),
        alternate_phone: Set(contact.alternate_phone().map(PhoneNumber::to_string)),
        email: Set(contact.email().map(Email::to_string)),
        address_line1: Set(address.line1().to_string()),
        address_line2: Set(address.line2().map(str::to_string)),
        city: Set(address.city().to_string()),
        province_code: Set(address.province().code().to_string()),
        country_code: Set(address.province().country().code().to_string()),
        postal_code: Set(address.postal_code().to_string()),
        status: Set(customer.status().label().to_string()),
        ..Default::default()
    }
}

/// **Create:** saves a new customer and returns them with the id the database gave. A customer
/// that already has an id is refused (use `update_customer`). A new customer is `Active` unless
/// they were built otherwise. (Two customers may share a name: telling them apart is for people.)
pub async fn create_customer(
    db: &impl ConnectionTrait,
    customer: &Customer,
) -> QuoteResult<Customer> {
    if customer.id().is_some() {
        return Err(QuoteError::CustomerAlreadySaved);
    }
    let saved = row_values(customer)
        .insert(db)
        .await
        .map_err(database_error)?;
    customer_from_row(saved)
}

/// **Read** one customer by the id the database gave.
pub async fn get_customer(db: &impl ConnectionTrait, id: Uuid) -> QuoteResult<Customer> {
    let row = Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or(QuoteError::UnknownCustomer)?;
    customer_from_row(row)
}

/// A quote keeps a copy of what this customer looked like the day it was quoted: the saved row as
/// JSON. This reads the customer and that copy together.
pub(crate) async fn get_customer_with_copy(
    db: &impl ConnectionTrait,
    id: Uuid,
) -> QuoteResult<(Customer, serde_json::Value)> {
    let row = Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or(QuoteError::UnknownCustomer)?;
    let copy = serde_json::to_value(&row).map_err(|_| QuoteError::StoredQuoteDamaged)?;
    Ok((customer_from_row(row)?, copy))
}

/// Builds the customer back from a quote's copy. Every rule is checked again, as for any saved row.
pub(crate) fn customer_from_copy(copy: &serde_json::Value) -> QuoteResult<Customer> {
    let row: row::Model =
        serde_json::from_value(copy.clone()).map_err(|_| QuoteError::StoredQuoteDamaged)?;
    customer_from_row(row)
}

/// **Read** a page of customers, ordered by name. `Some(Status::Active)` lists only those in use
/// (what a screen offers for a new quote); `None` lists all. A list of people can be long, so it
/// always comes in pages: `limit` customers, after the first `offset`.
pub async fn list_customers(
    db: &impl ConnectionTrait,
    status: Option<Status>,
    limit: u64,
    offset: u64,
) -> QuoteResult<Vec<Customer>> {
    let mut query = Entity::find()
        .order_by_asc(Column::Name)
        .order_by_asc(Column::Id); // so customers with the same name keep one fixed order
    if let Some(status) = status {
        query = query.filter(Column::Status.eq(status.label()));
    }
    query
        .limit(limit)
        .offset(offset)
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(customer_from_row)
        .collect()
}

/// **Update:** saves the new version of a customer that was loaded (it has an id). Build the new
/// version with `Customer::new(...)` and the old id (`with_id`) and status (`with_status`).
/// An unsaved customer is refused. Returns the customer as saved.
pub async fn update_customer(
    db: &impl ConnectionTrait,
    customer: &Customer,
) -> QuoteResult<Customer> {
    let id = customer.id().ok_or(QuoteError::CustomerNotSaved)?;
    if Entity::find_by_id(id)
        .count(db)
        .await
        .map_err(database_error)?
        == 0
    {
        return Err(QuoteError::UnknownCustomer);
    }
    let v = row_values(customer);
    Entity::update_many()
        .col_expr(Column::Name, Expr::value(v.name.unwrap()))
        .col_expr(Column::Company, Expr::value(v.company.unwrap()))
        .col_expr(Column::Designation, Expr::value(v.designation.unwrap()))
        .col_expr(Column::Phone, Expr::value(v.phone.unwrap()))
        .col_expr(
            Column::AlternatePhone,
            Expr::value(v.alternate_phone.unwrap()),
        )
        .col_expr(Column::Email, Expr::value(v.email.unwrap()))
        .col_expr(Column::AddressLine1, Expr::value(v.address_line1.unwrap()))
        .col_expr(Column::AddressLine2, Expr::value(v.address_line2.unwrap()))
        .col_expr(Column::City, Expr::value(v.city.unwrap()))
        .col_expr(Column::ProvinceCode, Expr::value(v.province_code.unwrap()))
        .col_expr(Column::CountryCode, Expr::value(v.country_code.unwrap()))
        .col_expr(Column::PostalCode, Expr::value(v.postal_code.unwrap()))
        .col_expr(Column::Status, Expr::value(v.status.unwrap()))
        .col_expr(Column::UpdatedAt, Expr::current_timestamp())
        .filter(Column::Id.eq(id))
        .exec(db)
        .await
        .map_err(database_error)?;
    get_customer(db, id).await
}

/// **"Delete":** a customer is never deleted. This switches them to `Inactive` (no new quotes,
/// old ones keep working) or back to `Active`. Returns the customer as saved.
pub async fn set_customer_status(
    db: &impl ConnectionTrait,
    id: Uuid,
    status: Status,
) -> QuoteResult<Customer> {
    Entity::update_many()
        .col_expr(Column::Status, Expr::value(status.label()))
        .col_expr(Column::UpdatedAt, Expr::current_timestamp())
        .filter(Column::Id.eq(id))
        .exec(db)
        .await
        .map_err(database_error)?;
    // An unknown id changes nothing, and reading it back says so (`UnknownCustomer`).
    get_customer(db, id).await
}
