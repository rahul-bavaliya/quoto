// src/repair_order/table.rs: saving and loading repair orders (the `repair_orders` and
// `repair_order_numbers` tables)
//
// What is saved, and why:
// - A repair order **points at its accepted quote** (`quote_id`, unique: one quote makes at most
//   one repair order) and at the quote's branch, customer and machine (copied ids, so a list can
//   be narrowed by them; an accepted quote never changes them). The lines, totals and parties are
//   read from the quote, which is final once accepted.
// - The number is handed out when the repair order is saved: one counter row per year, bumped in
//   the same transaction as the insert, so saved repair orders never have gaps.
// - **Two people at once:** like a quote, a repair order has a `version`; a change is accepted only
//   if it was made from the latest version, and the row is locked while saving.
// - Nothing is deleted: the database refuses to delete a quote, branch, customer or machine a
//   repair order points at.
// - Nothing here logs a value from a repair order (see `claude/logging-rules.md`).

use super::{RepairOrder, RepairOrderNumber, RepairOrderStatus, SavedRepairOrder};
use crate::equipment::MeterType;
use crate::errors::{QuoteError, QuoteResult};
use crate::quote::{QuoteStatus, QuoteTable, get_quote, quotes_by_ids};
use chrono::{Datelike, NaiveDateTime};
use sea_orm::sea_query::{Expr, ExprTrait, OnConflict};
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait};
use sea_orm::{QueryFilter, QueryOrder, QuerySelect, Set, TransactionSession, TransactionTrait};
use tracing::debug;
use uuid::Uuid;

pub(crate) use number_row::Entity as RepairOrderNumberTable;
pub(crate) use row::Entity as RepairOrderTable;

/// The biggest counter a year can reach (8 digits, see `RepairOrderNumber`).
const LAST_COUNTER: i32 = 99_999_999;

/// The `repair_orders` table as a struct.
mod row {
    use sea_orm::entity::prelude::*;

    #[sea_orm::model]
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "repair_orders")]
    pub struct Model {
        /// Made by the database, never by Quoto.
        #[sea_orm(
            primary_key,
            auto_increment = false,
            default_expr = "Expr::cust(\"gen_random_uuid()\")"
        )]
        pub id: Uuid,
        /// "RO-2026-00000001". Handed out when the repair order is saved. Never repeated.
        #[sea_orm(unique, column_type = "Text")]
        pub number: String,
        /// The accepted quote this was written up from. One quote, at most one repair order.
        #[sea_orm(unique)]
        pub quote_id: Uuid,
        /// Copied from the quote (an accepted quote never changes them) so lists can be narrowed.
        #[sea_orm(indexed)]
        pub branch_id: Uuid,
        #[sea_orm(indexed)]
        pub customer_id: Uuid,
        #[sea_orm(indexed)]
        pub equipment_id: Uuid,
        /// The shop's own date and time, as handed in (no time zone).
        pub created_at: DateTime,
        pub due_date: Option<Date>,
        /// "open", "in_progress", "completed" or "cancelled".
        #[sea_orm(indexed, column_type = "Text")]
        pub status: String,
        /// Filled exactly when the status is "cancelled".
        #[sea_orm(column_type = "Text")]
        pub cancellation_reason: Option<String>,
        /// The meter when the machine came in. Required.
        pub meter_reading: f64,
        /// "km", "mi" or "hr".
        #[sea_orm(column_type = "Text")]
        pub meter_type: String,
        pub started_at: Option<DateTime>,
        pub closed_at: Option<DateTime>,
        #[sea_orm(default_expr = "Expr::current_timestamp()")]
        pub updated_at: DateTimeUtc,
        /// 0 when made; +1 with every saved change (see `save_repair_order`).
        #[sea_orm(default_value = 0)]
        pub version: i32,
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// The `repair_order_numbers` table as a struct: the last counter given out in each year.
mod number_row {
    use sea_orm::entity::prelude::*;

    #[sea_orm::model]
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "repair_order_numbers")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub year: i16,
        pub last_number: i32,
    }

    impl ActiveModelBehavior for ActiveModel {}
}

fn database_error(cause: sea_orm::DbErr) -> QuoteError {
    // The cause helps whoever runs the server; it is not shown to the caller.
    debug!(%cause, "repair order query failed");
    QuoteError::DatabaseUnavailable
}

/// What a new repair order is made from: the accepted quote, given by id.
pub struct NewRepairOrder {
    /// The shop's date and time. It decides the year of the repair order number.
    pub created_at: NaiveDateTime,
    /// The accepted quote to write up.
    pub quote_id: Uuid,
    /// The meter when the machine came in. Required unless the quote already has a reading.
    pub meter_reading: Option<f64>,
}

/// What a list of repair orders is narrowed to. The default is everything.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RepairOrderFilter {
    pub customer_id: Option<Uuid>,
    pub branch_id: Option<Uuid>,
    pub equipment_id: Option<Uuid>,
    /// One of "open", "in_progress", "completed", "cancelled".
    pub status: Option<&'static str>,
}

/// Hands out the next repair order number of a year. It adds one to that year's counter row
/// (making the row if the year has none yet) and **holds the row until the surrounding
/// transaction ends**: if that transaction is undone, the number is given back. Call it inside
/// the transaction that saves the repair order (`create_repair_order` does).
pub async fn next_repair_order_number(
    db: &impl ConnectionTrait,
    year: u16,
) -> QuoteResult<RepairOrderNumber> {
    // A year that isn't a real 4-digit year is refused here, before anything is counted.
    RepairOrderNumber::new(year, 1)?;
    let counted = number_row::Entity::insert(number_row::ActiveModel {
        year: Set(
            i16::try_from(year).map_err(|_| QuoteError::RepairOrderNumbersExhausted { year })?
        ),
        last_number: Set(1),
    })
    .on_conflict(
        OnConflict::column(number_row::Column::Year)
            .value(
                number_row::Column::LastNumber,
                Expr::col((number_row::Entity, number_row::Column::LastNumber)).add(1),
            )
            .to_owned(),
    )
    .exec_with_returning(db)
    .await
    .map_err(database_error)?;
    if counted.last_number > LAST_COUNTER {
        return Err(QuoteError::RepairOrderNumbersExhausted { year });
    }
    RepairOrderNumber::new(
        year,
        u32::try_from(counted.last_number).map_err(|_| QuoteError::StoredRepairOrderDamaged)?,
    )
}

/// **Create** a repair order from an accepted quote: locks the quote's row (so two people
/// opening a repair order from the same quote take turns), checks the quote has none yet, checks
/// every rule (`RepairOrder::open`), hands out the number, and saves, all in one transaction.
/// Nothing is saved if any step fails, and the number is given back. Returns it as saved.
pub async fn create_repair_order<C>(db: &C, new: NewRepairOrder) -> QuoteResult<RepairOrder>
where
    C: ConnectionTrait + TransactionTrait,
{
    let year =
        u16::try_from(new.created_at.year()).map_err(|_| QuoteError::InvalidRepairOrderNumber {
            input: format!("year {}", new.created_at.year()),
        })?;
    let tx = db.begin().await.map_err(database_error)?;

    // While this row is locked nobody else can open a repair order from the same quote: the
    // second person waits, then finds the first one's repair order below.
    let quote_row = QuoteTable::find_by_id(new.quote_id)
        .lock_exclusive()
        .one(&tx)
        .await
        .map_err(database_error)?
        .ok_or(QuoteError::UnknownQuote)?;
    if row::Entity::find()
        .filter(row::Column::QuoteId.eq(new.quote_id))
        .count(&tx)
        .await
        .map_err(database_error)?
        > 0
    {
        return Err(QuoteError::RepairOrderExists);
    }
    let quote = get_quote(&tx, new.quote_id).await?;
    // Only an accepted quote has a customer and a machine, so these ids are there.
    let (Some(customer_id), Some(equipment_id)) = (quote_row.customer_id, quote_row.equipment_id)
    else {
        return Err(QuoteError::QuoteNotAccepted);
    };
    if quote_row.status != "accepted" {
        return Err(QuoteError::QuoteNotAccepted);
    }

    // The number comes last: the shortest time holding the year's counter.
    let number = next_repair_order_number(&tx, year).await?;
    let order = RepairOrder::open(number, new.created_at, quote, new.meter_reading)?;

    let saved = row::ActiveModel {
        number: Set(order.number().to_string()),
        quote_id: Set(new.quote_id),
        branch_id: Set(quote_row.branch_id),
        customer_id: Set(customer_id),
        equipment_id: Set(equipment_id),
        created_at: Set(order.created_at()),
        due_date: Set(order.due_date()),
        status: Set(order.status().label().to_string()),
        cancellation_reason: Set(None),
        meter_reading: Set(order.meter_reading()),
        meter_type: Set(order.meter_type().code().to_string()),
        started_at: Set(None),
        closed_at: Set(None),
        ..Default::default()
    }
    .insert(&tx)
    .await
    .map_err(database_error)?;

    let order = load(&tx, saved.id).await?;
    tx.commit().await.map_err(database_error)?;
    debug!("repair order saved");
    Ok(order)
}

/// **Read** one repair order by the id the database gave.
pub async fn get_repair_order(db: &impl ConnectionTrait, id: Uuid) -> QuoteResult<RepairOrder> {
    load(db, id).await
}

/// **Read** a page of repair orders, the newest first. A list can be long, so it always comes in
/// pages: `limit` repair orders, after the first `offset`.
pub async fn list_repair_orders(
    db: &impl ConnectionTrait,
    filter: &RepairOrderFilter,
    limit: u64,
    offset: u64,
) -> QuoteResult<Vec<RepairOrder>> {
    let mut query = row::Entity::find();
    if let Some(id) = filter.customer_id {
        query = query.filter(row::Column::CustomerId.eq(id));
    }
    if let Some(id) = filter.branch_id {
        query = query.filter(row::Column::BranchId.eq(id));
    }
    if let Some(id) = filter.equipment_id {
        query = query.filter(row::Column::EquipmentId.eq(id));
    }
    if let Some(status) = filter.status {
        query = query.filter(row::Column::Status.eq(status));
    }
    let rows = query
        .order_by_desc(row::Column::CreatedAt)
        .order_by_desc(row::Column::Number)
        .limit(limit)
        .offset(offset)
        .all(db)
        .await
        .map_err(database_error)?;

    // One query for the quotes of the whole page, not one per repair order.
    let ids: Vec<Uuid> = rows.iter().map(|row| row.quote_id).collect();
    let mut quotes = quotes_by_ids(db, &ids).await?;
    rows.into_iter()
        .map(|row| {
            let quote = quotes
                .remove(&row.quote_id)
                .ok_or(QuoteError::StoredRepairOrderDamaged)?;
            order_from_row(row, quote)
        })
        .collect()
}

/// **Update** a saved repair order with what changed on it: its status, its due date and the
/// times that go with a status. Nothing else about it can change.
///
/// **Two people at once:** a save is accepted only if the repair order was read at the latest
/// version; otherwise nothing is saved (`RepairOrderChangedMeanwhile`) and the caller reads it
/// again. The row is locked while saving, so two saves at the same instant take turns. Returns
/// the repair order as saved (the next version).
pub async fn save_repair_order<C>(db: &C, order: &RepairOrder) -> QuoteResult<RepairOrder>
where
    C: ConnectionTrait + TransactionTrait,
{
    let id = order.id().ok_or(QuoteError::RepairOrderNotSaved)?;
    let tx = db.begin().await.map_err(database_error)?;
    let stored = row::Entity::find_by_id(id)
        .lock_exclusive()
        .one(&tx)
        .await
        .map_err(database_error)?
        .ok_or(QuoteError::UnknownRepairOrder)?;
    if stored.version != order.version() {
        return Err(QuoteError::RepairOrderChangedMeanwhile);
    }

    let next_version = stored.version + 1;
    let mut update: row::ActiveModel = stored.into();
    update.version = Set(next_version);
    update.due_date = Set(order.due_date());
    update.status = Set(order.status().label().to_string());
    update.cancellation_reason = Set(cancellation_reason(order.status()));
    update.started_at = Set(order.started_at());
    update.closed_at = Set(order.closed_at());
    update.updated_at = Set(chrono::Utc::now());
    update.update(&tx).await.map_err(database_error)?;

    let order = load(&tx, id).await?;
    tx.commit().await.map_err(database_error)?;
    Ok(order)
}

fn cancellation_reason(status: &RepairOrderStatus) -> Option<String> {
    match status {
        RepairOrderStatus::Cancelled { reason } => Some(reason.clone()),
        _ => None,
    }
}

async fn load(db: &impl ConnectionTrait, id: Uuid) -> QuoteResult<RepairOrder> {
    let row = row::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or(QuoteError::UnknownRepairOrder)?;
    let quote = get_quote(db, row.quote_id).await?;
    order_from_row(row, quote)
}

/// Rebuilds a repair order from its row and its quote, checking everything again: a row damaged
/// behind Quoto's back is refused, not shown.
fn order_from_row(row: row::Model, quote: crate::quote::Quote) -> QuoteResult<RepairOrder> {
    let damaged = || QuoteError::StoredRepairOrderDamaged;
    let status = match (row.status.as_str(), row.cancellation_reason) {
        ("open", None) => RepairOrderStatus::Open,
        ("in_progress", None) => RepairOrderStatus::InProgress,
        ("completed", None) => RepairOrderStatus::Completed,
        ("cancelled", Some(reason)) => RepairOrderStatus::cancelled(&reason)?,
        _ => return Err(damaged()),
    };
    // The times must fit the status: started once work began, closed once it ended.
    let times_fit = match status {
        RepairOrderStatus::Open => row.started_at.is_none() && row.closed_at.is_none(),
        RepairOrderStatus::InProgress => row.started_at.is_some() && row.closed_at.is_none(),
        RepairOrderStatus::Completed => row.started_at.is_some() && row.closed_at.is_some(),
        RepairOrderStatus::Cancelled { .. } => row.closed_at.is_some(),
    };
    // The link and the quote must say the same thing.
    let same_quote = quote.id() == Some(row.quote_id)
        && matches!(quote.status(), QuoteStatus::Accepted)
        && quote.branch().id() == Some(row.branch_id)
        && quote.customer().map(|customer| customer.id()) == Some(Some(row.customer_id))
        && quote.equipment().map(|machine| machine.id()) == Some(Some(row.equipment_id));
    if !times_fit || !same_quote || !row.meter_reading.is_finite() || row.meter_reading < 0.0 {
        return Err(damaged());
    }
    Ok(RepairOrder::restore(SavedRepairOrder {
        id: row.id,
        version: row.version,
        number: RepairOrderNumber::parse(&row.number).map_err(|_| damaged())?,
        created_at: row.created_at,
        due_date: row.due_date,
        status,
        meter_reading: row.meter_reading,
        meter_type: MeterType::from_code(&row.meter_type).map_err(|_| damaged())?,
        started_at: row.started_at,
        closed_at: row.closed_at,
        quote,
    }))
}
