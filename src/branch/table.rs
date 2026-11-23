// src/branch/table.rs: the `branches` table in the database, and its CRUD
//
// Create, read and update are here. **There is no delete, on purpose**: a branch is never
// deleted (old quotes point at it). When it is not in use, `set_branch_status` makes it
// `Inactive`. The table is built by a migration; the struct in `row` says how it is read; the rules
// (a number is not blank, a phone and postal code are real, the map location is both numbers
// or neither, the status is a known word) are checked in Rust when a `Branch` is built and
// again when a row is read.

use super::{Branch, Location};
use crate::customer::{Address, PhoneNumber, PostalCode};
use crate::errors::{QuoteError, QuoteResult};
use crate::province::Province;
use crate::status::Status;
use sea_orm::sea_query::Expr;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait};
use sea_orm::{QueryFilter, QueryOrder, Set};
use tracing::debug;
use uuid::Uuid;

pub(crate) use row::Entity as BranchTable;
use row::{ActiveModel, Column, Entity};

/// The `branches` table as a struct.
mod row {
    use sea_orm::entity::prelude::*;

    #[sea_orm::model]
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize)]
    #[sea_orm(table_name = "branches")]
    pub struct Model {
        /// Made by the database, never by Quoto.
        #[sea_orm(
            primary_key,
            auto_increment = false,
            default_expr = "Expr::cust(\"gen_random_uuid()\")"
        )]
        pub id: Uuid,
        /// The number people use, e.g. "001". Text, so leading zeros survive. No two branches share one.
        #[sea_orm(unique, column_type = "Text")]
        pub number: String,
        #[sea_orm(column_type = "Text")]
        pub name: String,
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
        #[sea_orm(column_type = "Text")]
        pub phone: String,
        /// Both or neither (`Location` checks it).
        pub latitude: Option<f64>,
        pub longitude: Option<f64>,
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
    debug!(%cause, "branch query failed");
    QuoteError::DatabaseUnavailable
}

/// Rebuilds a `Branch` from a row, re-checking everything (a damaged row is refused).
fn branch_from_row(row: row::Model) -> QuoteResult<Branch> {
    let address = Address::new(
        row.address_line1,
        row.address_line2,
        row.city,
        PostalCode::parse(&row.postal_code)?,
        Province::from_code(&row.province_code)?,
    )?;
    let mut branch = Branch::new(
        row.number,
        row.name,
        address,
        PhoneNumber::parse(&row.phone)?,
    )?
    .with_id(row.id)
    .with_status(Status::from_label(&row.status)?);
    match (row.latitude, row.longitude) {
        (Some(latitude), Some(longitude)) => {
            branch = branch.with_location(Location::new(latitude, longitude)?);
        }
        (None, None) => {}
        _ => return Err(QuoteError::InvalidLocation), // only half a location
    }
    Ok(branch)
}

/// The row values of a branch (everything except the id and the two timestamps).
fn row_values(branch: &Branch) -> ActiveModel {
    let address = branch.address();
    ActiveModel {
        number: Set(branch.number().to_string()),
        name: Set(branch.name().to_string()),
        address_line1: Set(address.line1().to_string()),
        address_line2: Set(address.line2().map(str::to_string)),
        city: Set(address.city().to_string()),
        province_code: Set(branch.province().code().to_string()),
        country_code: Set(branch.country().code().to_string()),
        postal_code: Set(address.postal_code().to_string()),
        phone: Set(branch.phone().to_string()),
        latitude: Set(branch.location().map(Location::latitude)),
        longitude: Set(branch.location().map(Location::longitude)),
        status: Set(branch.status().label().to_string()),
        ..Default::default()
    }
}

/// Is another branch (not `except`) already using this number?
async fn number_taken(
    db: &impl ConnectionTrait,
    number: &str,
    except: Option<Uuid>,
) -> QuoteResult<bool> {
    let mut query = Entity::find().filter(Column::Number.eq(number));
    if let Some(id) = except {
        query = query.filter(Column::Id.ne(id));
    }
    Ok(query.count(db).await.map_err(database_error)? > 0)
}

/// **Create:** saves a new branch and returns it with the id the database gave it.
/// A branch that already has an id is refused (use `update_branch`), and so is a number that
/// another branch uses. A new branch is `Active` unless it was built otherwise.
pub async fn create_branch(db: &impl ConnectionTrait, branch: &Branch) -> QuoteResult<Branch> {
    if branch.id().is_some() {
        return Err(QuoteError::BranchAlreadySaved);
    }
    if number_taken(db, branch.number(), None).await? {
        return Err(QuoteError::DuplicateBranchNumber {
            number: branch.number().to_string(),
        });
    }
    let saved = row_values(branch)
        .insert(db)
        .await
        .map_err(database_error)?;
    branch_from_row(saved)
}

/// **Read** one branch by the id the database gave it.
pub async fn get_branch(db: &impl ConnectionTrait, id: Uuid) -> QuoteResult<Branch> {
    let row = Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or(QuoteError::UnknownBranch)?;
    branch_from_row(row)
}

/// A quote keeps a copy of what this branch looked like the day it was quoted: the saved row as
/// JSON. This reads the branch and that copy together.
/// **Read** one branch by the number people use ("001").
pub async fn branch_by_number(db: &impl ConnectionTrait, number: &str) -> QuoteResult<Branch> {
    let row = Entity::find()
        .filter(Column::Number.eq(number))
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or(QuoteError::UnknownBranch)?;
    branch_from_row(row)
}

/// **Read** the branches, ordered by number. `Some(Status::Active)` lists only those in use
/// (what a screen offers for a new quote); `None` lists all of them.
pub async fn list_branches(
    db: &impl ConnectionTrait,
    status: Option<Status>,
) -> QuoteResult<Vec<Branch>> {
    let mut query = Entity::find().order_by_asc(Column::Number);
    if let Some(status) = status {
        query = query.filter(Column::Status.eq(status.label()));
    }
    query
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(branch_from_row)
        .collect()
}

/// **Update:** saves the new version of a branch that was loaded (it has an id): rename it, move
/// it, give it a new phone. Build the new version with `Branch::new(...)` and the old id
/// (`with_id`), as `Branch` explains. An unsaved branch is refused, and so is a number that
/// another branch uses. Returns the branch as saved.
pub async fn update_branch(db: &impl ConnectionTrait, branch: &Branch) -> QuoteResult<Branch> {
    let id = branch.id().ok_or(QuoteError::BranchNotSaved)?;
    if Entity::find_by_id(id)
        .count(db)
        .await
        .map_err(database_error)?
        == 0
    {
        return Err(QuoteError::UnknownBranch);
    }
    if number_taken(db, branch.number(), Some(id)).await? {
        return Err(QuoteError::DuplicateBranchNumber {
            number: branch.number().to_string(),
        });
    }
    let v = row_values(branch);
    Entity::update_many()
        .col_expr(Column::Number, Expr::value(v.number.unwrap()))
        .col_expr(Column::Name, Expr::value(v.name.unwrap()))
        .col_expr(Column::AddressLine1, Expr::value(v.address_line1.unwrap()))
        .col_expr(Column::AddressLine2, Expr::value(v.address_line2.unwrap()))
        .col_expr(Column::City, Expr::value(v.city.unwrap()))
        .col_expr(Column::ProvinceCode, Expr::value(v.province_code.unwrap()))
        .col_expr(Column::CountryCode, Expr::value(v.country_code.unwrap()))
        .col_expr(Column::PostalCode, Expr::value(v.postal_code.unwrap()))
        .col_expr(Column::Phone, Expr::value(v.phone.unwrap()))
        .col_expr(Column::Latitude, Expr::value(v.latitude.unwrap()))
        .col_expr(Column::Longitude, Expr::value(v.longitude.unwrap()))
        .col_expr(Column::Status, Expr::value(v.status.unwrap()))
        .col_expr(Column::UpdatedAt, Expr::current_timestamp())
        .filter(Column::Id.eq(id))
        .exec(db)
        .await
        .map_err(database_error)?;
    get_branch(db, id).await
}

/// **"Delete":** a branch is never deleted. This switches it to `Inactive` (no new quotes, old
/// ones keep working) or back to `Active`. Returns the branch as saved.
pub async fn set_branch_status(
    db: &impl ConnectionTrait,
    id: Uuid,
    status: Status,
) -> QuoteResult<Branch> {
    Entity::update_many()
        .col_expr(Column::Status, Expr::value(status.label()))
        .col_expr(Column::UpdatedAt, Expr::current_timestamp())
        .filter(Column::Id.eq(id))
        .exec(db)
        .await
        .map_err(database_error)?;
    // An unknown id changes nothing, and reading it back says so (`UnknownBranch`).
    get_branch(db, id).await
}
