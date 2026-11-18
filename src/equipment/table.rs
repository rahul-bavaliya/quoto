// src/equipment/table.rs: the `equipment` table in the database, and its calls
//
// A machine is **never erased**: `set_equipment_status` archives it (`Inactive`) and restores it
// (`Active`), and everything that points at it stays valid. There is no SQL delete.
//
// A machine can be saved **with no customer** and given (or moved to) an owner later with
// `set_equipment_owner`: a sale changes the owner of the same record, so one physical machine
// stays one record. Only the current owner is kept (no ownership history yet). The owner must be
// an existing, active customer; customers are never deleted, so an owner id can't dangle.
//
// A serial number is unique (kept in capitals, see `Identification`); a plate is not, because
// plates move between vehicles. The rules (make and model not blank, serial or plate present,
// a valid meter, known types, a known source and status) are checked in Rust when an
// `Equipment` is built and again when a row is read.
//
// Serial numbers and plates identify one real vehicle: nothing here logs a value from the table
// (see `claude/logging-rules.md`), and only made-up machines may be saved.

use super::{Equipment, EquipmentSource, EquipmentType, Identification, Meter, MeterType};
use crate::customer::get_customer;
use crate::errors::{QuoteError, QuoteResult};
use crate::status::Status;
use sea_orm::sea_query::Expr;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait};
use sea_orm::{QueryFilter, QueryOrder, QuerySelect, Set};
use tracing::debug;
use uuid::Uuid;

pub(crate) use row::Entity as EquipmentTable;
use row::{ActiveModel, Column, Entity};

/// The `equipment` table as a struct.
mod row {
    use sea_orm::entity::prelude::*;

    #[sea_orm::model]
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize)]
    #[sea_orm(table_name = "equipment")]
    pub struct Model {
        /// Made by the database, never by Quoto.
        #[sea_orm(
            primary_key,
            auto_increment = false,
            default_expr = "Expr::cust(\"gen_random_uuid()\")"
        )]
        pub id: Uuid,
        /// The owner (a customer's id), or nothing yet. Changed only by `set_equipment_owner`.
        #[sea_orm(indexed)]
        pub customer_id: Option<Uuid>,
        #[sea_orm(column_type = "Text")]
        pub make: String,
        #[sea_orm(column_type = "Text")]
        pub model: String,
        pub model_year: Option<i64>,
        /// In capitals. Unique when there is one (the database lets many machines have none).
        #[sea_orm(unique, column_type = "Text")]
        pub serial_number: Option<String>,
        #[sea_orm(column_type = "Text")]
        pub plate: Option<String>,
        pub meter_reading: Option<f64>,
        /// "km", "mi" or "hr".
        #[sea_orm(column_type = "Text")]
        pub meter_type: String,
        /// "Car", "Truck", "Tractor", "Trailer" or "Other".
        #[sea_orm(column_type = "Text")]
        pub equipment_type: String,
        /// "manual" or "lookup:<provider>" (see `EquipmentSource`).
        #[sea_orm(column_type = "Text")]
        pub source: String,
        /// "active" or "inactive" (archived; see `Status`).
        #[sea_orm(column_type = "Text")]
        pub status: String,
        #[sea_orm(default_expr = "Expr::current_timestamp()")]
        pub created_at: DateTimeUtc,
        #[sea_orm(default_expr = "Expr::current_timestamp()")]
        pub updated_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// Whose machines a list shows.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum OwnerFilter {
    /// Everyone's, and machines with no owner.
    #[default]
    Any,
    /// Only machines with no owner yet.
    Unowned,
    /// Only the machines of this customer.
    Owned(Uuid),
}

/// What a list of machines is narrowed to. The default is everything.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EquipmentFilter {
    pub status: Option<Status>,
    pub owner: OwnerFilter,
    /// Find a machine by its serial number (any case, spaces around it ignored).
    pub serial_number: Option<String>,
}

/// A failed read or write: the cause goes to the debug log, the person sees a short message.
fn database_error(cause: sea_orm::DbErr) -> QuoteError {
    debug!(%cause, "equipment query failed");
    QuoteError::DatabaseUnavailable
}

/// Rebuilds an `Equipment` from a row, re-checking everything (a damaged row is refused).
fn equipment_from_row(row: row::Model) -> QuoteResult<Equipment> {
    let mut equipment = Equipment::new(
        row.make,
        row.model,
        row.model_year,
        Identification::new(row.serial_number, row.plate)?,
        Meter::new(row.meter_reading, MeterType::from_code(&row.meter_type)?)?,
        EquipmentType::from_label(&row.equipment_type)?,
    )?
    .with_id(row.id)
    .with_status(Status::from_label(&row.status)?);
    equipment.source = EquipmentSource::from_stored(&row.source)?;
    if let Some(owner) = row.customer_id {
        equipment = equipment.with_customer_id(owner);
    }
    Ok(equipment)
}

/// The row values of a machine (everything except the id and the two timestamps).
fn row_values(equipment: &Equipment) -> ActiveModel {
    ActiveModel {
        customer_id: Set(equipment.customer_id()),
        make: Set(equipment.make().to_string()),
        model: Set(equipment.model().to_string()),
        model_year: Set(equipment.model_year()),
        serial_number: Set(equipment
            .identification()
            .serial_number()
            .map(str::to_string)),
        plate: Set(equipment.identification().plate().map(str::to_string)),
        meter_reading: Set(equipment.meter().reading()),
        meter_type: Set(equipment.meter().meter_type().code().to_string()),
        equipment_type: Set(equipment.equipment_type().label().to_string()),
        source: Set(equipment.source().stored()),
        status: Set(equipment.status().label().to_string()),
        ..Default::default()
    }
}

/// An owner must be a customer who exists and is in use.
async fn check_owner(db: &impl ConnectionTrait, owner: Uuid) -> QuoteResult<()> {
    if get_customer(db, owner).await?.is_active() {
        Ok(())
    } else {
        Err(QuoteError::CustomerInactive)
    }
}

/// Is another machine (not `except`) already using this serial number?
async fn serial_taken(
    db: &impl ConnectionTrait,
    serial: &str,
    except: Option<Uuid>,
) -> QuoteResult<bool> {
    let mut query = Entity::find().filter(Column::SerialNumber.eq(serial));
    if let Some(id) = except {
        query = query.filter(Column::Id.ne(id));
    }
    Ok(query.count(db).await.map_err(database_error)? > 0)
}

/// **Create:** saves a new machine and returns it with the id the database gave. The owner is
/// optional (add one later with `set_equipment_owner`); when there is one it must be an existing,
/// active customer. A machine that already has an id is refused (use `update_equipment`), and so
/// is a serial number that another machine uses.
pub async fn create_equipment(
    db: &impl ConnectionTrait,
    equipment: &Equipment,
) -> QuoteResult<Equipment> {
    if equipment.id().is_some() {
        return Err(QuoteError::EquipmentAlreadySaved);
    }
    if let Some(owner) = equipment.customer_id() {
        check_owner(db, owner).await?;
    }
    if let Some(serial) = equipment.identification().serial_number()
        && serial_taken(db, serial, None).await?
    {
        return Err(QuoteError::DuplicateSerialNumber);
    }
    let saved = row_values(equipment)
        .insert(db)
        .await
        .map_err(database_error)?;
    equipment_from_row(saved)
}

/// **Read** one machine by the id the database gave.
pub async fn get_equipment(db: &impl ConnectionTrait, id: Uuid) -> QuoteResult<Equipment> {
    let row = Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or(QuoteError::UnknownEquipment)?;
    equipment_from_row(row)
}

/// A quote keeps a copy of what this equipment looked like the day it was quoted: the saved row as
/// JSON. This reads the equipment and that copy together.
pub(crate) async fn get_equipment_with_copy(
    db: &impl ConnectionTrait,
    id: Uuid,
) -> QuoteResult<(Equipment, serde_json::Value)> {
    let row = Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or(QuoteError::UnknownEquipment)?;
    let copy = serde_json::to_value(&row).map_err(|_| QuoteError::StoredQuoteDamaged)?;
    Ok((equipment_from_row(row)?, copy))
}

/// Builds the equipment back from a quote's copy. Every rule is checked again, as for any saved row.
pub(crate) fn equipment_from_copy(copy: &serde_json::Value) -> QuoteResult<Equipment> {
    let row: row::Model =
        serde_json::from_value(copy.clone()).map_err(|_| QuoteError::StoredQuoteDamaged)?;
    equipment_from_row(row)
}

/// **Read** a page of machines, ordered by make, model and serial number. Machines come in pages:
/// `limit` of them, after the first `offset`. A list for a new quote asks for `Status::Active`.
pub async fn list_equipment(
    db: &impl ConnectionTrait,
    filter: &EquipmentFilter,
    limit: u64,
    offset: u64,
) -> QuoteResult<Vec<Equipment>> {
    let mut query = Entity::find()
        .order_by_asc(Column::Make)
        .order_by_asc(Column::Model)
        .order_by_asc(Column::SerialNumber)
        .order_by_asc(Column::Id); // so machines that look alike keep one fixed order
    if let Some(status) = filter.status {
        query = query.filter(Column::Status.eq(status.label()));
    }
    match filter.owner {
        OwnerFilter::Any => {}
        OwnerFilter::Unowned => query = query.filter(Column::CustomerId.is_null()),
        OwnerFilter::Owned(customer) => query = query.filter(Column::CustomerId.eq(customer)),
    }
    if let Some(serial) = &filter.serial_number {
        query = query.filter(Column::SerialNumber.eq(serial.trim().to_uppercase()));
    }
    query
        .limit(limit)
        .offset(offset)
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(equipment_from_row)
        .collect()
}

/// **Update:** saves the new details of a machine that was loaded (it has an id): make, model,
/// year, serial number, plate, meter, type. Build the new version with `Equipment::new(...)` and
/// the old id (`with_id`). **The owner and the status are not changed here** (they have their own
/// calls), and neither is where the details came from. Returns the machine as saved.
pub async fn update_equipment(
    db: &impl ConnectionTrait,
    equipment: &Equipment,
) -> QuoteResult<Equipment> {
    let id = equipment.id().ok_or(QuoteError::EquipmentNotSaved)?;
    if Entity::find_by_id(id)
        .count(db)
        .await
        .map_err(database_error)?
        == 0
    {
        return Err(QuoteError::UnknownEquipment);
    }
    if let Some(serial) = equipment.identification().serial_number()
        && serial_taken(db, serial, Some(id)).await?
    {
        return Err(QuoteError::DuplicateSerialNumber);
    }
    let v = row_values(equipment);
    Entity::update_many()
        .col_expr(Column::Make, Expr::value(v.make.unwrap()))
        .col_expr(Column::Model, Expr::value(v.model.unwrap()))
        .col_expr(Column::ModelYear, Expr::value(v.model_year.unwrap()))
        .col_expr(Column::SerialNumber, Expr::value(v.serial_number.unwrap()))
        .col_expr(Column::Plate, Expr::value(v.plate.unwrap()))
        .col_expr(Column::MeterReading, Expr::value(v.meter_reading.unwrap()))
        .col_expr(Column::MeterType, Expr::value(v.meter_type.unwrap()))
        .col_expr(
            Column::EquipmentType,
            Expr::value(v.equipment_type.unwrap()),
        )
        .col_expr(Column::UpdatedAt, Expr::current_timestamp())
        .filter(Column::Id.eq(id))
        .exec(db)
        .await
        .map_err(database_error)?;
    get_equipment(db, id).await
}

/// **"Delete":** a machine is never erased. This archives it (`Inactive`: no new quotes, old ones
/// and everything else that points at it keep working) or restores it (`Active`).
/// Returns the machine as saved.
pub async fn set_equipment_status(
    db: &impl ConnectionTrait,
    id: Uuid,
    status: Status,
) -> QuoteResult<Equipment> {
    Entity::update_many()
        .col_expr(Column::Status, Expr::value(status.label()))
        .col_expr(Column::UpdatedAt, Expr::current_timestamp())
        .filter(Column::Id.eq(id))
        .exec(db)
        .await
        .map_err(database_error)?;
    // An unknown id changes nothing, and reading it back says so (`UnknownEquipment`).
    get_equipment(db, id).await
}

/// **Owner:** gives the machine to a customer (a sale moves it from one to another) or, with
/// `None`, to nobody. The new owner must be an existing, active customer. Only the current owner
/// is kept. Returns the machine as saved.
pub async fn set_equipment_owner(
    db: &impl ConnectionTrait,
    id: Uuid,
    owner: Option<Uuid>,
) -> QuoteResult<Equipment> {
    // Check the machine first, so an unknown machine is "unknown equipment" even with a bad owner.
    get_equipment(db, id).await?;
    if let Some(owner) = owner {
        check_owner(db, owner).await?;
    }
    Entity::update_many()
        .col_expr(Column::CustomerId, Expr::value(owner))
        .col_expr(Column::UpdatedAt, Expr::current_timestamp())
        .filter(Column::Id.eq(id))
        .exec(db)
        .await
        .map_err(database_error)?;
    get_equipment(db, id).await
}
