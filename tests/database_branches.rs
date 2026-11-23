// tests/database_branches.rs: saving and loading branches in a real database
//
// These need a running database, so they are skipped normally (and in CI). Run them by hand:
//
//     docker compose up -d
//     cargo test --test database_branches -- --ignored
//
// They build the tables first (safe to repeat). Every change a test makes happens inside a
// transaction that is dropped at the end, which undoes it, so no branch is left behind.
// The test branches use numbers like "T-01" so they never clash with branches you made yourself.

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
        String::from("1 Industrial Rd"),
        line2.map(String::from),
        String::from("Regina"),
        PostalCode::parse("S4P 3Y2").unwrap(),
        province,
    )
    .unwrap()
}

fn new_branch(number: &str, name: &str) -> Branch {
    Branch::new(
        String::from(number),
        String::from(name),
        address(Province::Saskatchewan, Some("Unit 4")),
        PhoneNumber::parse("306-555-0100").unwrap(),
    )
    .unwrap()
}

// ===== create =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_new_branch_is_saved_active_and_gets_an_id_from_the_database() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let saved = create_branch(&tx, &new_branch("T-01", "Test Regina"))
        .await
        .unwrap();
    assert!(saved.id().is_some());
    assert_eq!(saved.status(), Status::Active);
    assert!(saved.is_active());
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn every_field_comes_back_exactly_as_it_was_saved() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let original = new_branch("T-02", "Test Calgary")
        .with_location(Location::new(50.4452, -104.6189).unwrap());
    let saved = create_branch(&tx, &original).await.unwrap();
    let loaded = get_branch(&tx, saved.id().unwrap()).await.unwrap();

    assert_eq!(loaded, saved);
    assert_eq!(loaded.number(), "T-02");
    assert_eq!(loaded.name(), "Test Calgary");
    assert_eq!(loaded.address(), original.address()); // line 2, city, postal code, province
    assert_eq!(loaded.phone(), original.phone());
    assert_eq!(loaded.location(), original.location());
    assert_eq!(loaded.country(), Country::Canada);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_branch_without_a_second_address_line_or_a_location_comes_back_without_them() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let plain = Branch::new(
        "T-03".into(),
        "Plain".into(),
        address(Province::Alberta, None),
        PhoneNumber::parse("403-555-0100").unwrap(),
    )
    .unwrap();
    let saved = create_branch(&tx, &plain).await.unwrap();
    assert_eq!(saved.address().line2(), None);
    assert_eq!(saved.location(), None);
    assert_eq!(saved.province(), &Province::Alberta);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_second_branch_with_the_same_number_is_refused() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    create_branch(&tx, &new_branch("T-04", "First"))
        .await
        .unwrap();
    assert_eq!(
        create_branch(&tx, &new_branch("T-04", "Second")).await,
        Err(QuoteError::DuplicateBranchNumber {
            number: "T-04".into()
        })
    );
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_branch_that_already_has_an_id_cannot_be_created_again() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let saved = create_branch(&tx, &new_branch("T-05", "Once"))
        .await
        .unwrap();
    assert_eq!(
        create_branch(&tx, &saved).await,
        Err(QuoteError::BranchAlreadySaved)
    );
}

// ===== read =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_branch_is_found_by_its_number_and_an_unknown_one_is_refused() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let saved = create_branch(&tx, &new_branch("T-06", "Findable"))
        .await
        .unwrap();
    assert_eq!(branch_by_number(&tx, "T-06").await.unwrap(), saved);
    assert_eq!(
        branch_by_number(&tx, "T-nope").await,
        Err(QuoteError::UnknownBranch)
    );
    assert_eq!(
        get_branch(&tx, Uuid::nil()).await,
        Err(QuoteError::UnknownBranch)
    );
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_list_is_ordered_by_number_and_can_show_only_active_branches() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    create_branch(&tx, &new_branch("T-12", "B")).await.unwrap();
    let a = create_branch(&tx, &new_branch("T-11", "A")).await.unwrap();
    set_branch_status(&tx, a.id().unwrap(), Status::Inactive)
        .await
        .unwrap();

    let numbers = |branches: Vec<Branch>| -> Vec<String> {
        branches
            .iter()
            .map(|b| b.number().to_string())
            .filter(|n| n.starts_with("T-1"))
            .collect()
    };
    assert_eq!(
        numbers(list_branches(&tx, None).await.unwrap()),
        ["T-11", "T-12"]
    );
    assert_eq!(
        numbers(list_branches(&tx, Some(Status::Active)).await.unwrap()),
        ["T-12"]
    );
    assert_eq!(
        numbers(list_branches(&tx, Some(Status::Inactive)).await.unwrap()),
        ["T-11"]
    );
}

// ===== update =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_branch_can_be_renamed_moved_and_given_a_new_phone_and_keeps_its_id() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let saved = create_branch(&tx, &new_branch("T-07", "Old name"))
        .await
        .unwrap();
    let id = saved.id().unwrap();

    let newer = Branch::new(
        "T-07".into(),
        "New name".into(),
        address(Province::Alberta, None),
        PhoneNumber::parse("403-555-0199").unwrap(),
    )
    .unwrap()
    .with_location(Location::new(51.0, -114.0).unwrap())
    .with_id(id);
    let updated = update_branch(&tx, &newer).await.unwrap();

    assert_eq!(updated.id(), Some(id));
    assert_eq!(updated.name(), "New name");
    assert_eq!(updated.province(), &Province::Alberta);
    assert_eq!(updated.phone().to_string(), "+1-403-555-0199");
    assert!(updated.location().is_some());
    assert_eq!(get_branch(&tx, id).await.unwrap(), updated);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_map_location_can_be_removed_by_an_update() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let with_map = new_branch("T-08", "Mapped").with_location(Location::new(50.0, -104.0).unwrap());
    let saved = create_branch(&tx, &with_map).await.unwrap();
    let without = new_branch("T-08", "Mapped").with_id(saved.id().unwrap());
    assert_eq!(update_branch(&tx, &without).await.unwrap().location(), None);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn an_update_cannot_take_another_branchs_number_but_may_keep_its_own() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    create_branch(&tx, &new_branch("T-09", "Nine"))
        .await
        .unwrap();
    let ten = create_branch(&tx, &new_branch("T-10", "Ten"))
        .await
        .unwrap();

    let steals = new_branch("T-09", "Ten").with_id(ten.id().unwrap());
    assert_eq!(
        update_branch(&tx, &steals).await,
        Err(QuoteError::DuplicateBranchNumber {
            number: "T-09".into()
        })
    );
    let keeps = new_branch("T-10", "Ten renamed").with_id(ten.id().unwrap());
    assert!(update_branch(&tx, &keeps).await.is_ok());
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn an_unsaved_or_unknown_branch_cannot_be_updated() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    assert_eq!(
        update_branch(&tx, &new_branch("T-13", "Never saved")).await,
        Err(QuoteError::BranchNotSaved)
    );
    let ghost = new_branch("T-13", "Ghost").with_id(Uuid::nil());
    assert_eq!(
        update_branch(&tx, &ghost).await,
        Err(QuoteError::UnknownBranch)
    );
}

// ===== "delete" = change the status =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_branch_is_deactivated_and_reactivated_never_deleted() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let saved = create_branch(&tx, &new_branch("T-14", "Closing"))
        .await
        .unwrap();
    let id = saved.id().unwrap();

    let off = set_branch_status(&tx, id, Status::Inactive).await.unwrap();
    assert_eq!(off.status(), Status::Inactive);
    assert!(!off.is_active());
    // Still there, with everything else unchanged:
    assert_eq!(get_branch(&tx, id).await.unwrap().name(), "Closing");

    let on = set_branch_status(&tx, id, Status::Active).await.unwrap();
    assert_eq!(on.status(), Status::Active);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_status_of_an_unknown_branch_cannot_be_set() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    assert_eq!(
        set_branch_status(&tx, Uuid::nil(), Status::Inactive).await,
        Err(QuoteError::UnknownBranch)
    );
}

// ===== damaged rows are caught when read =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_row_with_only_half_a_location_or_an_unknown_status_is_refused_when_read() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let saved = create_branch(&tx, &new_branch("T-16", "Damaged"))
        .await
        .unwrap();
    let id = saved.id().unwrap();

    // Written behind Quoto's back (the table has no rules of its own).
    tx.execute_unprepared(&format!(
        "UPDATE branches SET latitude = 50.0 WHERE id = '{id}'"
    ))
    .await
    .unwrap();
    assert_eq!(get_branch(&tx, id).await, Err(QuoteError::InvalidLocation));

    tx.execute_unprepared(&format!(
        "UPDATE branches SET latitude = NULL, status = 'deleted' WHERE id = '{id}'"
    ))
    .await
    .unwrap();
    assert_eq!(
        get_branch(&tx, id).await,
        Err(QuoteError::InvalidStatus {
            value: "deleted".into()
        })
    );
}
