// tests/database_equipment.rs: saving and loading equipment in a real database
//
// These need a running database, so they are skipped normally (and in CI). Run them by hand:
//
//     docker compose up -d
//     cargo test --test database_equipment -- --ignored
//
// Every change a test makes happens inside a transaction that is dropped at the end, which
// undoes it, so no machine or customer is left behind. All machines and people here are made up.

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

/// A machine with every optional part filled in.
fn tractor(make: &str, serial: &str) -> Equipment {
    Equipment::new(
        make.to_string(),
        String::from("8R 310"),
        Some(2021),
        Identification::new(Some(serial.to_string()), Some(String::from("TR-1234"))).unwrap(),
        Meter::new(Some(1250.5), MeterType::Hour).unwrap(),
        EquipmentType::Tractor,
    )
    .unwrap()
}

/// A machine known only by its plate (no serial number).
fn by_plate(make: &str, plate: &str) -> Equipment {
    Equipment::new(
        make.to_string(),
        String::from("F-150"),
        None,
        Identification::new(None, Some(plate.to_string())).unwrap(),
        Meter::new(None, MeterType::Kilometer).unwrap(),
        EquipmentType::Truck,
    )
    .unwrap()
}

async fn customer(tx: &DatabaseTransaction) -> Customer {
    create_customer(tx, &samples::customer_in(Province::Saskatchewan).unwrap())
        .await
        .unwrap()
}

fn everything() -> EquipmentFilter {
    EquipmentFilter::default()
}

// ===== create and read =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_new_machine_is_saved_active_with_no_owner_and_an_id_from_the_database() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let saved = create_equipment(&tx, &tractor("T-JD", "t-serial-1"))
        .await
        .unwrap();
    assert!(saved.id().is_some());
    assert_eq!(saved.status(), Status::Active);
    assert_eq!(saved.customer_id(), None);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn every_field_comes_back_as_it_was_saved() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let saved = create_equipment(&tx, &tractor("T-JD", "t-serial-1 "))
        .await
        .unwrap();
    let loaded = get_equipment(&tx, saved.id().unwrap()).await.unwrap();
    assert_eq!(loaded, saved);
    assert_eq!(loaded.make(), "T-JD");
    assert_eq!(loaded.model(), "8R 310");
    assert_eq!(loaded.model_year(), Some(2021));
    assert_eq!(loaded.identification().serial_number(), Some("T-SERIAL-1")); // tidied
    assert_eq!(loaded.identification().plate(), Some("TR-1234"));
    assert_eq!(loaded.meter().reading(), Some(1250.5));
    assert_eq!(loaded.meter().meter_type(), MeterType::Hour);
    assert_eq!(loaded.equipment_type(), EquipmentType::Tractor);
    assert_eq!(loaded.source(), EquipmentSource::Manual);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_machine_with_only_a_plate_comes_back_without_the_rest() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let saved = create_equipment(&tx, &by_plate("T-Ford", "AB-1234"))
        .await
        .unwrap();
    assert_eq!(saved.identification().serial_number(), None);
    assert_eq!(saved.identification().plate(), Some("AB-1234"));
    assert_eq!(saved.model_year(), None);
    assert_eq!(saved.meter().reading(), None);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn where_the_details_came_from_is_kept() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let draft = EquipmentDraft::new(
        "NHTSA vPIC",
        String::from("T-Toyota"),
        String::from("Corolla"),
        Some(2025),
        EquipmentType::Car,
    );
    let machine = Equipment::from_draft(
        draft,
        Identification::new(Some(String::from("T-VIN-LOOKUP-1")), None).unwrap(),
        Meter::new(Some(10.0), MeterType::Kilometer).unwrap(),
    )
    .unwrap();
    let saved = create_equipment(&tx, &machine).await.unwrap();
    assert_eq!(saved.source(), EquipmentSource::Lookup("NHTSA vPIC"));
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_source_this_version_does_not_know_is_refused_not_guessed() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let draft = EquipmentDraft::new(
        "Some Other Service",
        String::from("T-Toyota"),
        String::from("Corolla"),
        None,
        EquipmentType::Car,
    );
    let machine = Equipment::from_draft(
        draft,
        Identification::new(Some(String::from("T-VIN-LOOKUP-2")), None).unwrap(),
        Meter::new(None, MeterType::Kilometer).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        create_equipment(&tx, &machine).await,
        Err(QuoteError::InvalidEquipmentSource)
    ));
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_machine_that_already_has_an_id_cannot_be_created_again() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let saved = create_equipment(&tx, &tractor("T-JD", "T-SERIAL-1"))
        .await
        .unwrap();
    assert!(matches!(
        create_equipment(&tx, &saved).await,
        Err(QuoteError::EquipmentAlreadySaved)
    ));
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_machine_nobody_has_is_unknown() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    assert!(matches!(
        get_equipment(&tx, Uuid::nil()).await,
        Err(QuoteError::UnknownEquipment)
    ));
}

// ===== one serial number, one machine =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_same_serial_number_cannot_be_saved_twice_whatever_the_case_or_spaces() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    create_equipment(&tx, &tractor("T-JD", "T-SERIAL-1"))
        .await
        .unwrap();
    for again in ["T-SERIAL-1", "t-serial-1", "  T-Serial-1  "] {
        assert!(
            matches!(
                create_equipment(&tx, &tractor("T-Other", again)).await,
                Err(QuoteError::DuplicateSerialNumber)
            ),
            "{again}"
        );
    }
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn machines_without_a_serial_and_machines_sharing_a_plate_are_fine() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    create_equipment(&tx, &by_plate("T-Ford", "SAME-1"))
        .await
        .unwrap();
    create_equipment(&tx, &by_plate("T-Ford", "SAME-1"))
        .await
        .unwrap(); // a plate is not unique
    create_equipment(&tx, &by_plate("T-Ford", "OTHER-2"))
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn an_update_cannot_take_another_machines_serial_but_may_keep_its_own() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    create_equipment(&tx, &tractor("T-JD", "T-SERIAL-1"))
        .await
        .unwrap();
    let second = create_equipment(&tx, &tractor("T-JD", "T-SERIAL-2"))
        .await
        .unwrap();
    let id = second.id().unwrap();
    assert!(matches!(
        update_equipment(&tx, &tractor("T-JD", "T-SERIAL-1").with_id(id)).await,
        Err(QuoteError::DuplicateSerialNumber)
    ));
    assert!(
        update_equipment(&tx, &tractor("T-JD", "t-serial-2").with_id(id))
            .await
            .is_ok()
    );
}

// ===== update =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn an_update_changes_the_details_and_keeps_the_owner_status_and_source() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let owner = customer(&tx).await;
    let saved = create_equipment(&tx, &tractor("T-JD", "T-SERIAL-1"))
        .await
        .unwrap();
    let id = saved.id().unwrap();
    set_equipment_owner(&tx, id, owner.id()).await.unwrap();
    set_equipment_status(&tx, id, Status::Inactive)
        .await
        .unwrap();

    let changed = by_plate("T-Renamed", "NEW-1").with_id(id);
    let updated = update_equipment(&tx, &changed).await.unwrap();
    assert_eq!(updated.make(), "T-Renamed");
    assert_eq!(updated.identification().serial_number(), None); // what was left out is cleared
    assert_eq!(updated.equipment_type(), EquipmentType::Truck);
    assert_eq!(updated.id(), Some(id));
    assert_eq!(updated.customer_id(), owner.id()); // not changed here
    assert_eq!(updated.status(), Status::Inactive); // not changed here
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn an_owner_carried_by_the_object_is_ignored_by_an_update() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let owner = customer(&tx).await;
    let saved = create_equipment(&tx, &tractor("T-JD", "T-SERIAL-1"))
        .await
        .unwrap();
    let id = saved.id().unwrap();
    let sneaky = tractor("T-JD", "T-SERIAL-1")
        .with_id(id)
        .with_customer_id(owner.id().unwrap());
    let updated = update_equipment(&tx, &sneaky).await.unwrap();
    assert_eq!(updated.customer_id(), None); // owners change only through set_equipment_owner
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn an_unsaved_or_unknown_machine_cannot_be_updated() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    assert!(matches!(
        update_equipment(&tx, &tractor("T-JD", "T-SERIAL-1")).await,
        Err(QuoteError::EquipmentNotSaved)
    ));
    let ghost = tractor("T-JD", "T-SERIAL-1").with_id(Uuid::nil());
    assert!(matches!(
        update_equipment(&tx, &ghost).await,
        Err(QuoteError::UnknownEquipment)
    ));
}

// ===== the owner =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_machine_can_be_saved_with_an_owner_and_the_owner_changes_when_it_is_sold() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let first = customer(&tx).await;
    let second = customer(&tx).await;
    let machine = tractor("T-JD", "T-SERIAL-1").with_customer_id(first.id().unwrap());
    let saved = create_equipment(&tx, &machine).await.unwrap();
    assert_eq!(saved.customer_id(), first.id());

    let id = saved.id().unwrap();
    let sold = set_equipment_owner(&tx, id, second.id()).await.unwrap();
    assert_eq!(sold.customer_id(), second.id());
    assert_eq!(sold.id(), Some(id)); // the same machine, the same record
    let loaded = get_equipment(&tx, id).await.unwrap();
    assert_eq!(loaded.customer_id(), second.id());
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn an_owner_can_be_added_later_and_taken_away_again() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let owner = customer(&tx).await;
    let saved = create_equipment(&tx, &tractor("T-JD", "T-SERIAL-1"))
        .await
        .unwrap();
    let id = saved.id().unwrap();
    let owned = set_equipment_owner(&tx, id, owner.id()).await.unwrap();
    assert_eq!(owned.customer_id(), owner.id());
    let free = set_equipment_owner(&tx, id, None).await.unwrap();
    assert_eq!(free.customer_id(), None);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_owner_must_be_a_customer_who_exists_and_is_in_use() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let closed = customer(&tx).await;
    set_customer_status(&tx, closed.id().unwrap(), Status::Inactive)
        .await
        .unwrap();
    let saved = create_equipment(&tx, &tractor("T-JD", "T-SERIAL-1"))
        .await
        .unwrap();
    let id = saved.id().unwrap();

    // on create
    let nobody = tractor("T-JD", "T-SERIAL-2").with_customer_id(Uuid::nil());
    assert!(matches!(
        create_equipment(&tx, &nobody).await,
        Err(QuoteError::UnknownCustomer)
    ));
    let shut = tractor("T-JD", "T-SERIAL-3").with_customer_id(closed.id().unwrap());
    assert!(matches!(
        create_equipment(&tx, &shut).await,
        Err(QuoteError::CustomerInactive)
    ));
    // on change
    assert!(matches!(
        set_equipment_owner(&tx, id, Some(Uuid::nil())).await,
        Err(QuoteError::UnknownCustomer)
    ));
    assert!(matches!(
        set_equipment_owner(&tx, id, closed.id()).await,
        Err(QuoteError::CustomerInactive)
    ));
    // nothing was saved by the refused calls
    assert_eq!(get_equipment(&tx, id).await.unwrap().customer_id(), None);
    assert!(
        list_equipment(
            &tx,
            &EquipmentFilter {
                serial_number: Some("T-SERIAL-2".into()),
                ..everything()
            },
            10,
            0
        )
        .await
        .unwrap()
        .is_empty()
    );
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_owner_of_a_machine_nobody_has_cannot_be_set() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let owner = customer(&tx).await;
    assert!(matches!(
        set_equipment_owner(&tx, Uuid::nil(), owner.id()).await,
        Err(QuoteError::UnknownEquipment)
    ));
}

// ===== archive and restore =====

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_machine_is_archived_and_restored_and_never_erased() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let saved = create_equipment(&tx, &tractor("T-JD", "T-SERIAL-1"))
        .await
        .unwrap();
    let id = saved.id().unwrap();
    let archived = set_equipment_status(&tx, id, Status::Inactive)
        .await
        .unwrap();
    assert_eq!(archived.status(), Status::Inactive);
    assert!(!archived.is_active());
    assert_eq!(archived.make(), "T-JD"); // nothing else changed
    // still there, and still the only holder of its serial number
    assert!(get_equipment(&tx, id).await.is_ok());
    assert!(matches!(
        create_equipment(&tx, &tractor("T-JD", "T-SERIAL-1")).await,
        Err(QuoteError::DuplicateSerialNumber)
    ));
    let restored = set_equipment_status(&tx, id, Status::Active).await.unwrap();
    assert_eq!(restored.status(), Status::Active);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn archiving_one_machine_leaves_the_others_alone() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let one = create_equipment(&tx, &tractor("T-JD", "T-SERIAL-1"))
        .await
        .unwrap();
    let two = create_equipment(&tx, &tractor("T-JD", "T-SERIAL-2"))
        .await
        .unwrap();
    set_equipment_status(&tx, one.id().unwrap(), Status::Inactive)
        .await
        .unwrap();
    assert_eq!(
        get_equipment(&tx, two.id().unwrap())
            .await
            .unwrap()
            .status(),
        Status::Active
    );
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn an_unknown_machine_cannot_be_archived() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    assert!(matches!(
        set_equipment_status(&tx, Uuid::nil(), Status::Inactive).await,
        Err(QuoteError::UnknownEquipment)
    ));
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn an_archived_machine_cannot_be_put_on_a_new_quote_but_a_restored_one_can() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let saved = create_equipment(&tx, &tractor("T-JD", "T-SERIAL-1"))
        .await
        .unwrap();
    let id = saved.id().unwrap();
    let archived = set_equipment_status(&tx, id, Status::Inactive)
        .await
        .unwrap();

    let new_quote = || {
        let mut quote = Quote::new(
            samples::quote_number(),
            samples::quote_time(),
            samples::branch_in(Province::Saskatchewan).unwrap(),
        )
        .unwrap();
        quote
            .attach_customer(samples::prairie_farms().unwrap())
            .unwrap();
        quote
    };
    let mut quote = new_quote();
    assert!(matches!(
        quote.attach_equipment(archived),
        Err(QuoteError::EquipmentInactive)
    ));
    assert!(quote.equipment().is_none());

    let restored = set_equipment_status(&tx, id, Status::Active).await.unwrap();
    let mut quote = new_quote();
    assert!(quote.attach_equipment(restored).is_ok()); // no owner on the machine: no clash
}

// ===== lists =====

async fn serials(tx: &DatabaseTransaction, filter: &EquipmentFilter) -> Vec<String> {
    list_equipment(tx, filter, 200, 0)
        .await
        .unwrap()
        .iter()
        .filter(|m| m.make().starts_with("T-"))
        .filter_map(|m| m.identification().serial_number().map(str::to_string))
        .collect()
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_list_is_ordered_by_make_then_model_then_serial() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    create_equipment(&tx, &tractor("T-B", "T-S2"))
        .await
        .unwrap();
    create_equipment(&tx, &tractor("T-A", "T-S3"))
        .await
        .unwrap();
    create_equipment(&tx, &tractor("T-B", "T-S1"))
        .await
        .unwrap();
    assert_eq!(serials(&tx, &everything()).await, ["T-S3", "T-S1", "T-S2"]);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_list_can_show_only_active_or_only_archived_machines() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    create_equipment(&tx, &tractor("T-A", "T-S1"))
        .await
        .unwrap();
    let gone = create_equipment(&tx, &tractor("T-A", "T-S2"))
        .await
        .unwrap();
    set_equipment_status(&tx, gone.id().unwrap(), Status::Inactive)
        .await
        .unwrap();
    let only = |status| EquipmentFilter {
        status: Some(status),
        ..everything()
    };
    assert_eq!(serials(&tx, &only(Status::Active)).await, ["T-S1"]);
    assert_eq!(serials(&tx, &only(Status::Inactive)).await, ["T-S2"]);
    assert_eq!(serials(&tx, &everything()).await, ["T-S1", "T-S2"]);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn the_list_can_show_one_customers_machines_or_the_ones_with_no_owner() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    let one = customer(&tx).await;
    let two = customer(&tx).await;
    for (serial, owner) in [
        ("T-S1", one.id()),
        ("T-S2", two.id()),
        ("T-S3", one.id()),
        ("T-S4", None),
    ] {
        let mut machine = tractor("T-A", serial);
        if let Some(owner) = owner {
            machine = machine.with_customer_id(owner);
        }
        create_equipment(&tx, &machine).await.unwrap();
    }
    let owned_by = |c: &Customer| EquipmentFilter {
        owner: OwnerFilter::Owned(c.id().unwrap()),
        ..everything()
    };
    assert_eq!(serials(&tx, &owned_by(&one)).await, ["T-S1", "T-S3"]);
    assert_eq!(serials(&tx, &owned_by(&two)).await, ["T-S2"]);
    let free = EquipmentFilter {
        owner: OwnerFilter::Unowned,
        ..everything()
    };
    assert_eq!(serials(&tx, &free).await, ["T-S4"]);
    assert_eq!(serials(&tx, &everything()).await.len(), 4);
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_machine_is_found_by_its_serial_number_in_any_case() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    create_equipment(&tx, &tractor("T-A", "T-FIND-ME"))
        .await
        .unwrap();
    create_equipment(&tx, &tractor("T-A", "T-OTHER"))
        .await
        .unwrap();
    for typed in ["T-FIND-ME", "t-find-me", "  T-Find-Me "] {
        let filter = EquipmentFilter {
            serial_number: Some(typed.into()),
            ..everything()
        };
        assert_eq!(serials(&tx, &filter).await, ["T-FIND-ME"], "{typed}");
    }
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn pages_line_up_with_the_whole_list() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    for n in 1..=5 {
        create_equipment(&tx, &tractor("T-A", &format!("T-S{n}")))
            .await
            .unwrap();
    }
    let all: Vec<Option<Uuid>> = list_equipment(&tx, &everything(), 200, 0)
        .await
        .unwrap()
        .iter()
        .map(|m| m.id())
        .collect();
    let page: Vec<Option<Uuid>> = list_equipment(&tx, &everything(), 2, 1)
        .await
        .unwrap()
        .iter()
        .map(|m| m.id())
        .collect();
    assert_eq!(page, all[1..3]);
}

// ===== a damaged row is refused, not shown =====

async fn damage(tx: &DatabaseTransaction, id: Uuid, change: &str) {
    tx.execute_unprepared(&format!("UPDATE equipment SET {change} WHERE id = '{id}'"))
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "needs the database (docker compose up)"]
async fn a_row_that_breaks_a_rule_is_refused_when_read() {
    let (_turn, db) = setup().await;
    let tx = work(&db).await;
    for (change, why) in [
        ("meter_type = 'parsecs'", "meter type"),
        ("equipment_type = 'spaceship'", "equipment type"),
        ("status = 'maybe'", "status"),
        ("source = 'somewhere'", "source"),
        ("meter_reading = -5", "meter reading"),
        ("make = '   '", "make"),
        ("serial_number = NULL, plate = NULL", "no identification"),
    ] {
        let saved = create_equipment(&tx, &tractor("T-JD", &format!("T-DAMAGE-{why}")))
            .await
            .unwrap();
        let id = saved.id().unwrap();
        damage(&tx, id, change).await;
        assert!(get_equipment(&tx, id).await.is_err(), "{why}");
        // Put it right again, so the next one starts from a good row (and the list stays readable).
        damage(&tx, id, "meter_type = 'hr', equipment_type = 'Tractor', status = 'active', source = 'manual', meter_reading = 1, make = 'T-JD', serial_number = 'T-FIXED-' || id::text, plate = 'X'").await;
    }
}
