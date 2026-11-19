// src/quote/table.rs: saving and loading quotes (the `quotes`, `quote_lines` and `quote_numbers`
// tables)
//
// What is saved, and why:
// - A quote **points at** its branch, customer and machine (`branch_id`, `customer_id`,
//   `equipment_id`) **and keeps a copy** of each as it was that day (the saved row as JSON), so
//   editing a customer later never changes an old quote. Loading a quote rebuilds the parties
//   from the copies, not from today's rows.
// - The quote **number is handed out when the quote is saved**: one counter row per year, bumped
//   in the same transaction as the insert. If the save fails the number goes back, so saved quotes
//   never have gaps (the cost: two quotes saved at the same moment take turns).
// - **Nothing is deleted.** A quote's lines are only added (the quote has no way to remove one);
//   the database refuses to delete a quote that has lines, or a branch, customer or machine a
//   quote points at.
// - The copies hold real names, phones and addresses: only made-up people until real login
//   exists, and nothing here logs a value from a quote (see `claude/logging-rules.md`).

use super::{LineItem, LineKind, Quote, QuoteNumber, QuoteStatus, SavedQuote};
use crate::branch::{branch_from_copy, get_branch_with_copy};
use crate::customer::{customer_from_copy, get_customer_with_copy};
use crate::equipment::{equipment_from_copy, get_equipment_with_copy};
use crate::errors::{QuoteError, QuoteResult};
use chrono::{Datelike, NaiveDateTime};
use sea_orm::sea_query::{Expr, ExprTrait, OnConflict};
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use sea_orm::{QueryOrder, QuerySelect, Set, TransactionSession, TransactionTrait};
use serde_json::Value;
use std::collections::HashMap;
use tracing::debug;
use uuid::Uuid;

pub(crate) use line_row::Entity as QuoteLineTable;
pub(crate) use number_row::Entity as QuoteNumberTable;
pub(crate) use row::Entity as QuoteTable;

/// The biggest counter a year can reach (8 digits, see `QuoteNumber`).
const LAST_COUNTER: i32 = 99_999_999;

/// The `quotes` table as a struct.
mod row {
    use sea_orm::entity::prelude::*;

    #[sea_orm::model]
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "quotes")]
    pub struct Model {
        /// Made by the database, never by Quoto.
        #[sea_orm(
            primary_key,
            auto_increment = false,
            default_expr = "Expr::cust(\"gen_random_uuid()\")"
        )]
        pub id: Uuid,
        /// "Q-2026-00000001". Handed out when the quote is saved. Never repeated.
        #[sea_orm(unique, column_type = "Text")]
        pub number: String,
        /// The shop's own date and time, as handed in (no time zone).
        pub created_at: DateTime,
        pub expires_on: Date,
        pub promise_date: Option<Date>,
        /// "pending", "accepted" or "rejected".
        #[sea_orm(column_type = "Text")]
        pub status: String,
        /// Filled exactly when the status is "rejected".
        #[sea_orm(column_type = "Text")]
        pub rejection_reason: Option<String>,
        #[sea_orm(indexed)]
        pub branch_id: Uuid,
        /// The branch's province the day the quote was made: it decides the taxes.
        #[sea_orm(column_type = "Text")]
        pub province_code: String,
        #[sea_orm(indexed)]
        pub customer_id: Option<Uuid>,
        #[sea_orm(indexed)]
        pub equipment_id: Option<Uuid>,
        /// What the branch, customer and machine looked like the day they were put on the quote.
        #[sea_orm(column_type = "JsonBinary")]
        pub branch_copy: Json,
        #[sea_orm(column_type = "JsonBinary")]
        pub customer_copy: Option<Json>,
        #[sea_orm(column_type = "JsonBinary")]
        pub equipment_copy: Option<Json>,
        #[sea_orm(default_expr = "Expr::current_timestamp()")]
        pub updated_at: DateTimeUtc,
        /// 0 when the quote is made; +1 with every saved change (see `save_quote`).
        #[sea_orm(default_value = 0)]
        pub version: i32,
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// The `quote_lines` table as a struct.
mod line_row {
    use sea_orm::entity::prelude::*;

    #[sea_orm::model]
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "quote_lines")]
    pub struct Model {
        #[sea_orm(
            primary_key,
            auto_increment = false,
            default_expr = "Expr::cust(\"gen_random_uuid()\")"
        )]
        pub id: Uuid,
        /// The quote this line belongs to; with `position` it is unique.
        #[sea_orm(unique_key = "quote_position")]
        pub quote_id: Uuid,
        /// 0 for the first line added, then 1, 2... (a table has no order of its own).
        #[sea_orm(unique_key = "quote_position")]
        pub position: i32,
        /// "LABOUR", "PART", "FEE" or "CREDIT".
        #[sea_orm(column_type = "Text")]
        pub kind: String,
        #[sea_orm(column_type = "Text")]
        pub description: String,
        pub quantity: f64,
        /// As entered, never negative (a credit is made negative by its kind).
        pub unit_price_cents: i64,
        #[sea_orm(default_expr = "Expr::current_timestamp()")]
        pub created_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// The `quote_numbers` table as a struct: the last counter given out in each year.
mod number_row {
    use sea_orm::entity::prelude::*;

    #[sea_orm::model]
    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "quote_numbers")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub year: i16,
        pub last_number: i32,
    }

    impl ActiveModelBehavior for ActiveModel {}
}

fn database_error(cause: sea_orm::DbErr) -> QuoteError {
    // The cause helps whoever runs the server; it is not shown to the caller.
    debug!(%cause, "quote query failed");
    QuoteError::DatabaseUnavailable
}

/// What a new quote is made from. The branch, customer and machine are given by id: they are
/// read from the database, checked (in use, the machine is the customer's...), and copied.
pub struct NewQuote {
    /// The shop's date and time. It decides the year of the quote number.
    pub created_at: NaiveDateTime,
    pub branch_id: Uuid,
    /// Empty on a quick quote (a walk-in or a phone call).
    pub customer_id: Option<Uuid>,
    /// Empty until the writer adds the machine. A machine needs a customer.
    pub equipment_id: Option<Uuid>,
    pub items: Vec<LineItem>,
}

/// What a list of quotes is narrowed to. The default is everything.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct QuoteFilter {
    pub customer_id: Option<Uuid>,
    pub branch_id: Option<Uuid>,
    pub equipment_id: Option<Uuid>,
}

/// Hands out the next quote number of a year. It adds one to that year's counter row (making the
/// row if the year has none yet) and **holds the row until the surrounding transaction ends**: if
/// that transaction is undone, the number is given back. Call it inside the transaction that
/// saves the quote (`create_quote` does).
pub async fn next_quote_number(db: &impl ConnectionTrait, year: u16) -> QuoteResult<QuoteNumber> {
    // A year that isn't a real 4-digit year is refused here, before anything is counted.
    QuoteNumber::first_of_year(year)?;
    let counted = number_row::Entity::insert(number_row::ActiveModel {
        year: Set(i16::try_from(year).map_err(|_| QuoteError::QuoteNumbersExhausted { year })?),
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
        return Err(QuoteError::QuoteNumbersExhausted { year });
    }
    QuoteNumber::new(
        year,
        u32::try_from(counted.last_number).map_err(|_| QuoteError::StoredQuoteDamaged)?,
    )
}

/// **Create** a quote: reads the branch (and the customer and machine, when given), checks every
/// rule a quote has (`Quote::new`, `attach_customer`, `attach_equipment`, `add_item`), hands out
/// the number, and saves the quote with its lines and the copies, all in one transaction. Nothing
/// is saved if any step fails, and the number is given back. Returns the quote as saved.
pub async fn create_quote<C>(db: &C, new: NewQuote) -> QuoteResult<Quote>
where
    C: ConnectionTrait + TransactionTrait,
{
    let year =
        u16::try_from(new.created_at.year()).map_err(|_| QuoteError::InvalidQuoteNumber {
            input: format!("year {}", new.created_at.year()),
        })?;
    let tx = db.begin().await.map_err(database_error)?;

    let (branch, branch_copy) = get_branch_with_copy(&tx, new.branch_id).await?;
    let customer = match new.customer_id {
        Some(id) => Some(get_customer_with_copy(&tx, id).await?),
        None => None,
    };
    let equipment = match new.equipment_id {
        Some(id) => Some(get_equipment_with_copy(&tx, id).await?),
        None => None,
    };

    // The number comes last: the shortest time holding the year's counter.
    let number = next_quote_number(&tx, year).await?;
    let mut quote = Quote::new(number, new.created_at, branch)?;
    let customer_copy = match customer {
        Some((customer, copy)) => {
            quote.attach_customer(customer)?;
            Some(copy)
        }
        None => None,
    };
    let equipment_copy = match equipment {
        Some((equipment, copy)) => {
            quote.attach_equipment(equipment)?;
            Some(copy)
        }
        None => None,
    };
    for item in new.items {
        quote.add_item(item)?;
    }

    let saved = row::ActiveModel {
        number: Set(quote.number().to_string()),
        created_at: Set(quote.created_at()),
        expires_on: Set(quote.expires_on()),
        promise_date: Set(quote.promise_date()),
        status: Set(quote.status().label().to_string()),
        rejection_reason: Set(rejection_reason(quote.status())),
        branch_id: Set(new.branch_id),
        province_code: Set(quote.province().code().to_string()),
        customer_id: Set(new.customer_id),
        equipment_id: Set(new.equipment_id),
        branch_copy: Set(branch_copy),
        customer_copy: Set(customer_copy),
        equipment_copy: Set(equipment_copy),
        ..Default::default()
    }
    .insert(&tx)
    .await
    .map_err(database_error)?;
    insert_new_lines(&tx, saved.id, &quote, 0).await?;

    let quote = load(&tx, saved.id).await?;
    tx.commit().await.map_err(database_error)?;
    debug!("quote saved");
    Ok(quote)
}

/// **Read** one quote by the id the database gave, with its lines in the order they were added.
pub async fn get_quote(db: &impl ConnectionTrait, id: Uuid) -> QuoteResult<Quote> {
    load(db, id).await
}

/// **Read** a page of quotes, the newest first. A list can be long, so it always comes in pages:
/// `limit` quotes, after the first `offset`.
pub async fn list_quotes(
    db: &impl ConnectionTrait,
    filter: &QuoteFilter,
    limit: u64,
    offset: u64,
) -> QuoteResult<Vec<Quote>> {
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
    let rows = query
        .order_by_desc(row::Column::CreatedAt)
        .order_by_desc(row::Column::Number)
        .limit(limit)
        .offset(offset)
        .all(db)
        .await
        .map_err(database_error)?;

    // One query for the lines of the whole page, not one per quote.
    let ids: Vec<Uuid> = rows.iter().map(|row| row.id).collect();
    let mut lines = line_rows(db, &ids).await?;
    rows.into_iter()
        .map(|row| {
            let own = lines.remove(&row.id).unwrap_or_default();
            quote_from_rows(row, own)
        })
        .collect()
}

/// **Update** a saved quote with what changed on it: its status (accepted, rejected, reopened),
/// its dates, the customer or machine added to a quick quote, and **new** lines. Lines already
/// saved are never touched. A customer or machine added now is read from the database and copied
/// now; one already on the quote keeps its old copy.
///
/// **Two people at once:** every quote has a `version` (0 when made, +1 with every saved change).
/// A save is accepted only if the quote was read at the latest version; otherwise nothing is saved
/// (`QuoteChangedMeanwhile`) and the caller reads it again. The row is locked while saving, so
/// even two saves at the same instant take turns. Returns the quote as saved (the next version).
pub async fn save_quote<C>(db: &C, quote: &Quote) -> QuoteResult<Quote>
where
    C: ConnectionTrait + TransactionTrait,
{
    let id = quote.id().ok_or(QuoteError::QuoteNotSaved)?;
    let tx = db.begin().await.map_err(database_error)?;
    // The row is locked until this save ends, so two saves of the same quote take turns: the
    // second one waits, then sees the first one's new version and is refused below.
    let stored = row::Entity::find_by_id(id)
        .lock_exclusive()
        .one(&tx)
        .await
        .map_err(database_error)?
        .ok_or(QuoteError::UnknownQuote)?;
    if stored.version != quote.version() {
        return Err(QuoteError::QuoteChangedMeanwhile);
    }

    let (customer_id, customer_copy) = match party_change(
        stored.customer_id,
        quote.customer().map(|customer| customer.id()),
        QuoteError::CustomerNotSaved,
    )? {
        PartyChange::Keep => (stored.customer_id, stored.customer_copy.clone()),
        PartyChange::Add(id) => {
            let (customer, copy) = get_customer_with_copy(&tx, id).await?;
            if !customer.is_active() {
                return Err(QuoteError::CustomerInactive);
            }
            (Some(id), Some(copy))
        }
    };
    let (equipment_id, equipment_copy) = match party_change(
        stored.equipment_id,
        quote.equipment().map(|equipment| equipment.id()),
        QuoteError::EquipmentNotSaved,
    )? {
        PartyChange::Keep => (stored.equipment_id, stored.equipment_copy.clone()),
        PartyChange::Add(id) => {
            let (equipment, copy) = get_equipment_with_copy(&tx, id).await?;
            if !equipment.is_active() {
                return Err(QuoteError::EquipmentInactive);
            }
            (Some(id), Some(copy))
        }
    };

    // The same version means the quote was read after the last change, so the lines it knows are
    // exactly the lines saved, and the new ones go after them.
    let known = quote
        .items()
        .iter()
        .filter(|item| item.id().is_some())
        .count();

    let next_version = stored.version + 1;
    let mut update: row::ActiveModel = stored.into();
    update.version = Set(next_version);
    update.expires_on = Set(quote.expires_on());
    update.promise_date = Set(quote.promise_date());
    update.status = Set(quote.status().label().to_string());
    update.rejection_reason = Set(rejection_reason(quote.status()));
    update.customer_id = Set(customer_id);
    update.customer_copy = Set(customer_copy);
    update.equipment_id = Set(equipment_id);
    update.equipment_copy = Set(equipment_copy);
    update.updated_at = Set(chrono::Utc::now());
    update.update(&tx).await.map_err(database_error)?;
    insert_new_lines(&tx, id, quote, known).await?;

    let quote = load(&tx, id).await?;
    tx.commit().await.map_err(database_error)?;
    Ok(quote)
}

/// What a save does about a quote's customer (or machine).
enum PartyChange {
    /// Nothing changes: still none, or the same one with its old copy.
    Keep,
    /// Added now to a quote that had none: read it and copy it as it is today.
    Add(Uuid),
}

/// Checks the change of a quote's customer (or machine): one may be added to a quote that had
/// none, and it is never swapped or removed. `on_quote` is `None` when the quote has none; the
/// inner `None` is one that has not been saved (it has no id).
fn party_change(
    stored: Option<Uuid>,
    on_quote: Option<Option<Uuid>>,
    not_saved: QuoteError,
) -> QuoteResult<PartyChange> {
    match (stored, on_quote) {
        (None, None) => Ok(PartyChange::Keep),
        (None, Some(Some(id))) => Ok(PartyChange::Add(id)),
        (None, Some(None)) => Err(not_saved),
        (Some(old), Some(Some(now))) if old == now => Ok(PartyChange::Keep),
        // Someone else added or changed it since this quote was read.
        (Some(_), _) => Err(QuoteError::QuoteChangedMeanwhile),
    }
}

fn rejection_reason(status: &QuoteStatus) -> Option<String> {
    match status {
        QuoteStatus::Rejected { reason } => Some(reason.clone()),
        _ => None,
    }
}

/// Saves the lines of `quote` that have no id yet. The line at index `i` of the quote goes at
/// position `i`, so `already_saved` (how many are in the table) is where the new ones start.
async fn insert_new_lines(
    db: &impl ConnectionTrait,
    quote_id: Uuid,
    quote: &Quote,
    already_saved: usize,
) -> QuoteResult<()> {
    for (position, item) in quote.items().iter().enumerate().skip(already_saved) {
        let position = i32::try_from(position).map_err(|_| QuoteError::StoredQuoteDamaged)?;
        line_row::ActiveModel {
            quote_id: Set(quote_id),
            position: Set(position),
            kind: Set(item.kind().label().to_string()),
            description: Set(item.description().to_string()),
            quantity: Set(item.quantity()),
            unit_price_cents: Set(item.unit_price_cents()),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(database_error)?;
    }
    Ok(())
}

/// The saved lines of these quotes, each quote's lines in the order they were added.
async fn line_rows(
    db: &impl ConnectionTrait,
    quote_ids: &[Uuid],
) -> QuoteResult<HashMap<Uuid, Vec<line_row::Model>>> {
    let rows = line_row::Entity::find()
        .filter(line_row::Column::QuoteId.is_in(quote_ids.iter().copied()))
        .order_by_asc(line_row::Column::QuoteId)
        .order_by_asc(line_row::Column::Position)
        .all(db)
        .await
        .map_err(database_error)?;
    let mut by_quote: HashMap<Uuid, Vec<line_row::Model>> = HashMap::new();
    for row in rows {
        by_quote.entry(row.quote_id).or_default().push(row);
    }
    Ok(by_quote)
}

async fn load(db: &impl ConnectionTrait, id: Uuid) -> QuoteResult<Quote> {
    let row = row::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or(QuoteError::UnknownQuote)?;
    let lines = line_rows(db, &[id]).await?.remove(&id).unwrap_or_default();
    quote_from_rows(row, lines)
}

/// Rebuilds a quote from its row and its lines, checking everything again: a row damaged behind
/// Quoto's back is refused, not shown.
fn quote_from_rows(row: row::Model, lines: Vec<line_row::Model>) -> QuoteResult<Quote> {
    let status = match (row.status.as_str(), row.rejection_reason) {
        ("pending", None) => QuoteStatus::Pending,
        ("accepted", None) => QuoteStatus::Accepted,
        ("rejected", Some(reason)) => QuoteStatus::rejected(&reason)?,
        _ => return Err(QuoteError::StoredQuoteDamaged),
    };
    let branch = branch_from_copy(&row.branch_copy)?;
    // The copy and the link must say the same thing.
    if branch.id() != Some(row.branch_id) || branch.province().code() != row.province_code {
        return Err(QuoteError::StoredQuoteDamaged);
    }
    let customer = party_from_copy(row.customer_id, row.customer_copy, |copy| {
        customer_from_copy(copy).map(|customer| customer.id())
    })?
    .map(|copy| customer_from_copy(&copy))
    .transpose()?;
    let equipment = party_from_copy(row.equipment_id, row.equipment_copy, |copy| {
        equipment_from_copy(copy).map(|equipment| equipment.id())
    })?
    .map(|copy| equipment_from_copy(&copy))
    .transpose()?;

    let mut items = Vec::with_capacity(lines.len());
    for (index, line) in lines.into_iter().enumerate() {
        // Positions are 0, 1, 2... with no gap: a missing one means a line went missing.
        if usize::try_from(line.position).ok() != Some(index) {
            return Err(QuoteError::StoredQuoteDamaged);
        }
        let kind = LineKind::from_stored(&line.kind).ok_or(QuoteError::StoredQuoteDamaged)?;
        items.push(
            LineItem::new(line.description, line.quantity, line.unit_price_cents, kind)?
                .with_id(line.id),
        );
    }

    Ok(Quote::restore(SavedQuote {
        id: row.id,
        version: row.version,
        number: QuoteNumber::parse(&row.number)?,
        created_at: row.created_at,
        expires_on: row.expires_on,
        promise_date: row.promise_date,
        status,
        branch,
        customer,
        equipment,
        items,
    }))
}

/// A party's link and copy must be both there or both empty, and the copy must be of the party
/// the link points at. Returns the copy when there is one.
fn party_from_copy(
    link: Option<Uuid>,
    copy: Option<Value>,
    id_in_copy: impl Fn(&Value) -> QuoteResult<Option<Uuid>>,
) -> QuoteResult<Option<Value>> {
    match (link, copy) {
        (None, None) => Ok(None),
        (Some(link), Some(copy)) if id_in_copy(&copy)? == Some(link) => Ok(Some(copy)),
        _ => Err(QuoteError::StoredQuoteDamaged),
    }
}
