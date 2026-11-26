// src/customer/table.rs: the `customers` table in the database, and its CRUD.
//
// Create, read, update and set status are here. **There is no delete, on purpose**:
// a customer is never deleted (historical quotes point at it). When not in use,
// `set_customer_status` sets it to `Inactive`.

use super::Customer;
use crate::Language;
use crate::common::{Address, Contact, Email, PhoneNumber, PostalCode};
use crate::customer_address::CustomerAddress;
use crate::errors::{AppError, AppResult};
use crate::province_country::ProvinceCountry;
use crate::status::Status;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use std::str::FromStr;
use tracing::debug;
use uuid::Uuid;

// pub(crate) use row::Entity as CustomerTable;
use row::{ActiveModel, Column, Entity};

/// The `customers` table as a struct.
mod row {
    use sea_orm::entity::prelude::*;

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
        #[sea_orm(default_value = false)]
        pub is_business: bool,
        #[sea_orm(column_type = "Text")]
        pub business_name: Option<String>,
        #[sea_orm(column_type = "Text")]
        pub customer_name: String,

        // Mailing Address
        #[sea_orm(column_type = "Text")]
        pub mail_address_line1: String,
        #[sea_orm(column_type = "Text")]
        pub mail_address_line2: Option<String>,
        #[sea_orm(column_type = "Text")]
        pub mail_city: String,
        /// "SK", "AB"... (the province's code; its taxes live in the `provinces` table)
        #[sea_orm(column_type = "Text")]
        pub mail_province_code: String,
        /// Written from the province ("CA"), derived from `ProvinceCountry`.
        #[sea_orm(column_type = "Text")]
        pub mail_country_code: String,
        #[sea_orm(column_type = "Text")]
        pub mail_postal_code: String,
        #[sea_orm(default_value = true)]
        pub use_shipping_as_mailing: bool,

        // Shipping Address
        #[sea_orm(column_type = "Text")]
        pub ship_address_line1: Option<String>,
        #[sea_orm(column_type = "Text")]
        pub ship_address_line2: Option<String>,
        #[sea_orm(column_type = "Text")]
        pub ship_city: Option<String>,
        /// "SK", "AB"... (the province's code; its taxes live in the `provinces` table)
        #[sea_orm(column_type = "Text")]
        pub ship_province_code: Option<String>,
        /// Written from the province ("CA"), derived from `ProvinceCountry`.
        #[sea_orm(column_type = "Text")]
        pub ship_country_code: Option<String>,
        #[sea_orm(column_type = "Text")]
        pub ship_postal_code: Option<String>,

        #[sea_orm(column_type = "Text")]
        pub phone1: String,
        #[sea_orm(column_type = "Text")]
        pub phone2: Option<String>,
        #[sea_orm(column_type = "Text")]
        pub email: String,

        #[sea_orm(column_type = "Text", default_value = "EN")]
        pub language: Option<String>,

        #[sea_orm(column_type = "Text")]
        pub gst_registration_no: Option<String>,

        #[sea_orm(column_type = "Text")]
        pub insurance_exp_date: Option<DateTimeUtc>,

        /// "active" or "inactive" (see `Status`).
        #[sea_orm(column_type = "Text")]
        pub status: String,
        #[sea_orm(default_expr = "Expr::current_timestamp()")]
        pub created_at: DateTimeUtc,
        #[sea_orm(default_expr = "Expr::current_timestamp()")]
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// A failed read or write: the cause goes to the debug log, the person sees a short message.
fn database_error(cause: sea_orm::DbErr) -> AppError {
    debug!(%cause, "Customer query failed");
    AppError::DatabaseUnavailable
}

/// Rebuilds a `Customer` from a row, re-checking everything (a damaged row is refused).
fn customer_from_row(row: row::Model) -> AppResult<Customer> {
    let mailing_address = Address::new(
        row.mail_address_line1,
        row.mail_address_line2,
        row.mail_city,
        PostalCode::parse(&row.mail_postal_code)?,
        ProvinceCountry::from_province_code(&row.mail_province_code)?,
    )?;

    let shipping_address: Option<Address> = if row.use_shipping_as_mailing {
        None
    } else {
        let line1 = row.ship_address_line1.ok_or(AppError::MissingField {
            field: "shipping address line 1",
        })?;
        let city = row.ship_city.ok_or(AppError::MissingField {
            field: "shipping city",
        })?;
        let province_code = row
            .ship_province_code
            .as_deref()
            .ok_or(AppError::MissingField {
                field: "shipping province",
            })?;
        let postal_code_raw = row
            .ship_postal_code
            .as_deref()
            .ok_or(AppError::MissingField {
                field: "shipping postal code",
            })?;

        Some(Address::new(
            line1,
            row.ship_address_line2,
            city,
            PostalCode::parse(postal_code_raw)?,
            ProvinceCountry::from_province_code(province_code)?,
        )?)
    };

    let customer_address = CustomerAddress::new(
        mailing_address,
        row.use_shipping_as_mailing,
        shipping_address,
    )?;

    let phone1 = PhoneNumber::parse(&row.phone1)?;
    let phone2 = row.phone2.as_deref().map(PhoneNumber::parse).transpose()?;
    let email = Email::parse(&row.email)?;
    let language = Language::from_str(row.language.as_deref().unwrap_or("EN"))?;
    let contact = Contact::new(phone1, phone2, email, Some(language))?;

    let mut customer = Customer::new(row.customer_name, customer_address, contact)?;
    if row.is_business {
        let biz_name = row.business_name.ok_or(AppError::MissingField {
            field: "business name",
        })?;
        customer = customer.as_business(biz_name)?;
    }

    Ok(customer
        .with_gst_registration_no(row.gst_registration_no)
        .with_id(row.id)
        .with_status(Status::from_label(&row.status)?)
        .with_timestamps(Some(row.created_at), Some(row.updated_at)))
}

/// The row values of a customer (everything except the id and the two timestamps).
fn row_values(customer: &Customer) -> ActiveModel {
    let mailing = customer.mailing_address();
    let shipping = customer.shipping_address();
    let contact = customer.contact();

    ActiveModel {
        is_business: Set(customer.is_business()),
        business_name: Set(customer.business_name().map(str::to_owned)),
        customer_name: Set(customer.customer_name().to_owned()),

        // Mailing address
        mail_address_line1: Set(mailing.line1().to_owned()),
        mail_address_line2: Set(mailing.line2().map(str::to_owned)),
        mail_city: Set(mailing.city().to_owned()),
        mail_province_code: Set(mailing.province_country().province_code().to_owned()),
        mail_country_code: Set(mailing.province_country().country_code().to_owned()),
        mail_postal_code: Set(mailing.postal_code().to_string()),

        use_shipping_as_mailing: Set(customer.use_shipping_as_mailing()),

        // Shipping address (populated from effective shipping address)
        ship_address_line1: Set(Some(shipping.line1().to_owned())),
        ship_address_line2: Set(shipping.line2().map(str::to_owned)),
        ship_city: Set(Some(shipping.city().to_owned())),
        ship_province_code: Set(Some(shipping.province_country().province_code().to_owned())),
        ship_country_code: Set(Some(shipping.province_country().country_code().to_owned())),
        ship_postal_code: Set(Some(shipping.postal_code().to_string())),

        // Contact
        phone1: Set(contact.phone1().to_string()),
        phone2: Set(contact.phone2().map(ToString::to_string)),
        email: Set(contact.email().to_string()),
        language: Set(contact
            .language()
            .map(|lang| lang.code().to_string())
            .or_else(|| Some("EN".to_string()))),

        // Tax & lifecycle
        gst_registration_no: Set(customer.gst_registration_no().map(str::to_owned)),
        status: Set(customer.status().label().to_owned()),
        ..Default::default()
    }
}

/// **Create:** saves a new customer and returns it with the id the database gave it.
/// A customer that already has an id is refused (use `update_customer`).
pub async fn create_customer(
    db: &impl ConnectionTrait,
    customer: &Customer,
) -> AppResult<Customer> {
    if customer.id().is_some() {
        return Err(AppError::CustomerAlreadySaved);
    }
    let saved = row_values(customer)
        .insert(db)
        .await
        .map_err(database_error)?;
    customer_from_row(saved)
}

/// **Read** one customer by the id the database gave it.
pub async fn get_customer(db: &impl ConnectionTrait, id: Uuid) -> AppResult<Customer> {
    let row = Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or(AppError::UnknownCustomer)?;
    customer_from_row(row)
}

/// **Read** customers, ordered by customer name. `Some(Status::Active)` lists only active ones;
/// `None` lists all of them.
pub async fn list_customers(
    db: &impl ConnectionTrait,
    status: Option<Status>,
) -> AppResult<Vec<Customer>> {
    let mut query = Entity::find().order_by_asc(Column::CustomerName);
    if let Some(status) = status {
        query = query.filter(Column::Status.eq(status.label()));
    }
    query
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(customer_from_row)
        .collect()
}

/// **Update:** saves the new version of a customer that was loaded (it has an id).
/// An unsaved customer is refused. Returns the customer as saved.
pub async fn update_customer(
    db: &impl ConnectionTrait,
    customer: &Customer,
) -> AppResult<Customer> {
    let id = customer.id().ok_or(AppError::CustomerNotSaved)?;
    let _existing = get_customer(db, id).await?;

    let v = row_values(customer);
    Entity::update_many()
        .col_expr(Column::IsBusiness, Expr::value(v.is_business.unwrap()))
        .col_expr(Column::BusinessName, Expr::value(v.business_name.unwrap()))
        .col_expr(Column::CustomerName, Expr::value(v.customer_name.unwrap()))
        .col_expr(
            Column::MailAddressLine1,
            Expr::value(v.mail_address_line1.unwrap()),
        )
        .col_expr(
            Column::MailAddressLine2,
            Expr::value(v.mail_address_line2.unwrap()),
        )
        .col_expr(Column::MailCity, Expr::value(v.mail_city.unwrap()))
        .col_expr(
            Column::MailProvinceCode,
            Expr::value(v.mail_province_code.unwrap()),
        )
        .col_expr(
            Column::MailCountryCode,
            Expr::value(v.mail_country_code.unwrap()),
        )
        .col_expr(
            Column::MailPostalCode,
            Expr::value(v.mail_postal_code.unwrap()),
        )
        .col_expr(
            Column::UseShippingAsMailing,
            Expr::value(v.use_shipping_as_mailing.unwrap()),
        )
        .col_expr(
            Column::ShipAddressLine1,
            Expr::value(v.ship_address_line1.unwrap()),
        )
        .col_expr(
            Column::ShipAddressLine2,
            Expr::value(v.ship_address_line2.unwrap()),
        )
        .col_expr(Column::ShipCity, Expr::value(v.ship_city.unwrap()))
        .col_expr(
            Column::ShipProvinceCode,
            Expr::value(v.ship_province_code.unwrap()),
        )
        .col_expr(
            Column::ShipCountryCode,
            Expr::value(v.ship_country_code.unwrap()),
        )
        .col_expr(
            Column::ShipPostalCode,
            Expr::value(v.ship_postal_code.unwrap()),
        )
        .col_expr(Column::Phone1, Expr::value(v.phone1.unwrap()))
        .col_expr(Column::Phone2, Expr::value(v.phone2.unwrap()))
        .col_expr(Column::Email, Expr::value(v.email.unwrap()))
        .col_expr(Column::Language, Expr::value(v.language.unwrap()))
        .col_expr(
            Column::GstRegistrationNo,
            Expr::value(v.gst_registration_no.unwrap()),
        )
        .col_expr(Column::Status, Expr::value(v.status.unwrap()))
        .col_expr(Column::UpdatedAt, Expr::current_timestamp())
        .filter(Column::Id.eq(id))
        .exec(db)
        .await
        .map_err(database_error)?;

    get_customer(db, id).await
}

/// **"Delete":** a customer is never deleted. Switches status between `Active` and `Inactive`.
pub async fn set_customer_status(
    db: &impl ConnectionTrait,
    id: Uuid,
    status: Status,
) -> AppResult<Customer> {
    Entity::update_many()
        .col_expr(Column::Status, Expr::value(status.label()))
        .col_expr(Column::UpdatedAt, Expr::current_timestamp())
        .filter(Column::Id.eq(id))
        .exec(db)
        .await
        .map_err(database_error)?;

    get_customer(db, id).await
}
