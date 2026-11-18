// tests/database_customers.rs: saving and loading customers in a real database
//
// These need a running database, so they are skipped normally (and in CI). Run them by hand:
//
//     docker compose up -d
//     cargo test --test database_customers -- --ignored
//
// Every change a test makes happens inside a transaction that is dropped at the end, which
// undoes it, so no customer is left behind. All people here are made up.

use quoto::*;
use sea_orm::{ConnectionTrait, DatabaseConnection, DatabaseTransaction, TransactionTrait};
use uuid::Uuid;

/// The tests take turns: two of them building the tables at the same moment would collide.
static ONE_AT_A_TIME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn setup() -> (tokio::sync::MutexGuard<'static, ()>, DatabaseConnection) {
    let turn = ONE_AT_A_TIME.lock().await;
    dotenvy::dotenv().ok();
    let settings = DatabaseSettings::from_env().expect("database settings (see .env.example)");
    let db = database::connect(&settings).await.expect("database");
    database::migrate(&db).await.expect("migrate");
    (turn, db)
}

/// A transaction that is never committed: dropping it undoes everything.
async fn work(db: &DatabaseConnection) -> DatabaseTransaction {
    db.begin().await.expect("transaction")
}

fn address(province: Province, line2: Option<&str>) -> Address {
    Address::new(
        String::from("123 Main St"),
        line2.map(String::from),
        String::from("Regina"),
        PostalCode::parse("S4P 3Y2").unwrap(),
        province,
    )
    .unwrap()
}

/// A customer with every optional part filled in.
fn full(name: &str) -> Customer {
    Customer::new(
        name.to_string(),
        Some(String::from("Prairie Farms Ltd.")),
        Some(String::from("Service Manager")),
        Contact::new(
            PhoneNumber::parse("306-555-0142").unwrap(),
            Some(PhoneNumber::parse("306-555-0143").unwrap()),
            Some(Email::parse("Jane@PrairieFarms.ca").unwrap()),
        ),
        address(Province::Saskatchewan, Some("Suite 2")),
    )
    .unwrap()
}

/// A customer with only what is required.
fn plain(name: &str) -> Customer {
    Customer::new(
        name.to_string(),
        None,
        None,
        Contact::new(PhoneNumber::parse("403-555-0100").unwrap(), None, None),
        address(Province::Alberta, None),
    )
    .unwrap()
}

// ===== create and read =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_new_customer_is_saved_active_with_an_id_from_the_database() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let saved = create_customer(&tx, &full("T-Jane")).await.unwrap();
    assert!(saved.id().is_some());
    assert_eq!(saved.status(), Status::Active);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn every_field_comes_back_as_it_was_saved() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let original = full("T-Jane");
    let saved = create_customer(&tx, &original).await.unwrap();
    let loaded = get_customer(&tx, saved.id().unwrap()).await.unwrap();

    assert_eq!(loaded, saved);
    assert_eq!(loaded.name(), "T-Jane");
    assert_eq!(loaded.company(), Some("Prairie Farms Ltd."));
    assert_eq!(loaded.designation(), Some("Service Manager"));
    assert_eq!(loaded.contact(), original.contact());
    assert_eq!(loaded.address(), original.address());
    assert_eq!(
        loaded.contact().email().unwrap().to_string(),
        "jane@prairiefarms.ca"
    );
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_customer_with_only_the_required_parts_comes_back_without_the_rest() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let saved = create_customer(&tx, &plain("T-Plain")).await.unwrap();
    assert_eq!(saved.company(), None);
    assert_eq!(saved.designation(), None);
    assert_eq!(saved.contact().alternate_phone(), None);
    assert_eq!(saved.contact().email(), None);
    assert_eq!(saved.address().line2(), None);
    assert_eq!(saved.address().province(), &Province::Alberta);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn two_customers_may_share_a_name_and_are_told_apart_by_id() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let one = create_customer(&tx, &plain("T-Same Name")).await.unwrap();
    let two = create_customer(&tx, &plain("T-Same Name")).await.unwrap();
    assert_ne!(one.id(), two.id());
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_customer_that_already_has_an_id_cannot_be_created_again() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let saved = create_customer(&tx, &plain("T-Once")).await.unwrap();
    assert_eq!(
        create_customer(&tx, &saved).await,
        Err(QuoteError::CustomerAlreadySaved)
    );
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn an_unknown_customer_is_refused() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    assert_eq!(
        get_customer(&tx, Uuid::nil()).await,
        Err(QuoteError::UnknownCustomer)
    );
}

// ===== lists =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_list_is_ordered_by_name_and_can_show_only_one_status() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    create_customer(&tx, &plain("T-Bob")).await.unwrap();
    let ann = create_customer(&tx, &plain("T-Ann")).await.unwrap();
    set_customer_status(&tx, ann.id().unwrap(), Status::Inactive)
        .await
        .unwrap();

    let mine = |customers: Vec<Customer>| -> Vec<String> {
        customers
            .iter()
            .map(|c| c.name().to_string())
            .filter(|n| n.starts_with("T-"))
            .collect()
    };
    let all = list_customers(&tx, None, 1000, 0).await.unwrap();
    assert_eq!(mine(all), ["T-Ann", "T-Bob"]);
    let active = list_customers(&tx, Some(Status::Active), 1000, 0)
        .await
        .unwrap();
    assert_eq!(mine(active), ["T-Bob"]);
    let inactive = list_customers(&tx, Some(Status::Inactive), 1000, 0)
        .await
        .unwrap();
    assert_eq!(mine(inactive), ["T-Ann"]);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_list_comes_in_pages_that_line_up() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    for name in ["T-P1", "T-P2", "T-P3", "T-P4"] {
        create_customer(&tx, &plain(name)).await.unwrap();
    }
    let ids = |customers: Vec<Customer>| -> Vec<Option<Uuid>> {
        customers.iter().map(Customer::id).collect()
    };
    let everything = ids(list_customers(&tx, None, 1000, 0).await.unwrap());
    assert!(everything.len() >= 4);

    let first = ids(list_customers(&tx, None, 2, 0).await.unwrap());
    let second = ids(list_customers(&tx, None, 2, 2).await.unwrap());
    assert_eq!(first, everything[0..2]);
    assert_eq!(second, everything[2..4]);
    assert!(
        list_customers(&tx, None, 5, 1_000_000)
            .await
            .unwrap()
            .is_empty()
    );
}

// ===== update =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_customer_can_be_changed_and_keeps_the_id() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let saved = create_customer(&tx, &full("T-Old Name")).await.unwrap();
    let id = saved.id().unwrap();

    let newer = plain("T-New Name").with_id(id);
    let updated = update_customer(&tx, &newer).await.unwrap();

    assert_eq!(updated.id(), Some(id));
    assert_eq!(updated.name(), "T-New Name");
    assert_eq!(updated.company(), None); // an empty part replaces the old one
    assert_eq!(updated.contact().email(), None);
    assert_eq!(updated.address().province(), &Province::Alberta);
    assert_eq!(get_customer(&tx, id).await.unwrap(), updated);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn an_update_carries_the_status_it_is_given() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let saved = create_customer(&tx, &plain("T-Closing")).await.unwrap();
    let id = saved.id().unwrap();
    set_customer_status(&tx, id, Status::Inactive)
        .await
        .unwrap();

    let kept = plain("T-Closing renamed")
        .with_id(id)
        .with_status(Status::Inactive);
    assert_eq!(
        update_customer(&tx, &kept).await.unwrap().status(),
        Status::Inactive
    );
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn an_unsaved_or_unknown_customer_cannot_be_updated() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    assert_eq!(
        update_customer(&tx, &plain("T-Never saved")).await,
        Err(QuoteError::CustomerNotSaved)
    );
    let ghost = plain("T-Ghost").with_id(Uuid::nil());
    assert_eq!(
        update_customer(&tx, &ghost).await,
        Err(QuoteError::UnknownCustomer)
    );
}

// ===== "delete" = change the status =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_customer_is_deactivated_and_reactivated_never_deleted() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let saved = create_customer(&tx, &full("T-Leaving")).await.unwrap();
    let id = saved.id().unwrap();

    let off = set_customer_status(&tx, id, Status::Inactive)
        .await
        .unwrap();
    assert_eq!(off.status(), Status::Inactive);
    assert_eq!(get_customer(&tx, id).await.unwrap().name(), "T-Leaving"); // still there

    let on = set_customer_status(&tx, id, Status::Active).await.unwrap();
    assert_eq!(on.status(), Status::Active);
    assert_eq!(on, saved);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_status_of_an_unknown_customer_cannot_be_set() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    assert_eq!(
        set_customer_status(&tx, Uuid::nil(), Status::Inactive).await,
        Err(QuoteError::UnknownCustomer)
    );
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn an_inactive_customer_still_loads_but_cannot_go_on_a_new_quote() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let saved = create_customer(&tx, &plain("T-Inactive")).await.unwrap();
    let off = set_customer_status(&tx, saved.id().unwrap(), Status::Inactive)
        .await
        .unwrap();

    let branch = samples::branch_in(Province::Saskatchewan).unwrap();
    let mut quote = Quote::new(samples::quote_number(), samples::quote_time(), branch).unwrap();
    assert_eq!(
        quote.attach_customer(off),
        Err(QuoteError::CustomerInactive)
    );
}

// ===== damaged rows are caught when read =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_row_with_a_bad_status_phone_or_province_is_refused_when_read() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let saved = create_customer(&tx, &plain("T-Damaged")).await.unwrap();
    let id = saved.id().unwrap();
    let damage = |sql: &'static str| {
        let tx = &tx;
        async move {
            tx.execute_unprepared(&format!("{sql} WHERE id = '{id}'"))
                .await
                .unwrap();
        }
    };

    // Written behind Quoto's back (the table has no rules of its own).
    damage("UPDATE customers SET status = 'deleted'").await;
    assert_eq!(
        get_customer(&tx, id).await,
        Err(QuoteError::InvalidStatus {
            value: "deleted".into()
        })
    );
    damage("UPDATE customers SET status = 'active', phone = 'abc'").await;
    assert_eq!(
        get_customer(&tx, id).await,
        Err(PhoneNumber::parse("abc").unwrap_err())
    );
    damage("UPDATE customers SET phone = '+1-403-555-0100', province_code = 'ZZ'").await;
    assert_eq!(
        get_customer(&tx, id).await,
        Err(QuoteError::UnknownProvince { code: "ZZ".into() })
    );
}
