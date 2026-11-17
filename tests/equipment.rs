// tests/equipment.rs: tests for the equipment feature (public API only)

use quoto::*;

// ===== equipment =====

fn corolla() -> Equipment {
    Equipment::new(
        String::from("Toyota"),
        String::from("Corolla Cross"),
        Some(2025),
        Identification::new(
            Some(String::from("SERIAL_NUMBER_TEST_1")),
            Some(String::from("RB97")),
        )
        .unwrap(),
        Meter::new(Some(28156.98), MeterType::Kilometer).unwrap(),
        EquipmentType::Car,
    )
    .unwrap()
}

#[test]
fn equipment_shows_all_its_details() {
    assert_eq!(
        corolla().to_string(),
        "Toyota Corolla Cross (2025)\nS/N: SERIAL_NUMBER_TEST_1\nMeter: 28156.98 km\nPlate: RB97"
    );
}

#[test]
fn equipment_without_optional_details_skips_them() {
    let tractor = Equipment::new(
        String::from("John Deere"),
        String::from("8R 410"),
        None,
        Identification::new(Some(String::from("1RW8R410ABC")), None).unwrap(),
        Meter::new(None, MeterType::Hour).unwrap(),
        EquipmentType::Tractor,
    )
    .unwrap();
    assert_eq!(
        tractor.to_string(),
        "John Deere 8R 410\nS/N: 1RW8R410ABC\nMeter: not recorded"
    );
}

// ---- Equipment::new refuses blanks ----

#[test]
fn equipment_with_a_blank_make_is_refused() {
    let result = Equipment::new(
        String::from("  "),
        String::from("Corolla"),
        None,
        Identification::new(None, Some(String::from("RB97"))).unwrap(),
        Meter::new(None, MeterType::Kilometer).unwrap(),
        EquipmentType::Car,
    );
    assert_eq!(result, Err(QuoteError::MissingField { field: "make" }));
}

#[test]
fn equipment_with_a_blank_model_is_refused() {
    let result = Equipment::new(
        String::from("Toyota"),
        String::new(),
        None,
        Identification::new(None, Some(String::from("RB97"))).unwrap(),
        Meter::new(None, MeterType::Kilometer).unwrap(),
        EquipmentType::Car,
    );
    assert_eq!(result, Err(QuoteError::MissingField { field: "model" }));
}

#[test]
fn equipment_type_has_a_label() {
    assert_eq!(EquipmentType::Truck.label(), "Truck");
    assert_eq!(corolla().equipment_type(), EquipmentType::Car);
}

// ===== identification =====

#[test]
fn identification_with_neither_serial_nor_plate_is_refused() {
    assert_eq!(
        Identification::new(None, None),
        Err(QuoteError::MissingIdentification)
    );
}

#[test]
fn identification_with_only_blank_text_is_refused() {
    assert!(Identification::new(Some(String::from("  ")), Some(String::new())).is_err());
}

#[test]
fn identification_with_only_a_serial_is_accepted() {
    assert!(Identification::new(Some(String::from("ABC123")), None).is_ok());
}

#[test]
fn identification_with_only_a_plate_is_accepted() {
    assert!(Identification::new(None, Some(String::from("RB97"))).is_ok());
}

// ===== meter =====

#[test]
fn two_meters_with_the_same_reading_are_equal() {
    assert_eq!(
        Meter::new(Some(100.0), MeterType::Hour).unwrap(),
        Meter::new(Some(100.0), MeterType::Hour).unwrap()
    );
    assert_ne!(
        Meter::new(Some(100.0), MeterType::Hour).unwrap(),
        Meter::new(Some(100.0), MeterType::Mile).unwrap()
    );
}

#[test]
fn a_meter_without_a_reading_is_allowed_on_a_quote() {
    let meter = Meter::new(None, MeterType::Hour).unwrap();
    assert_eq!(meter.to_string(), "not recorded");
}

#[test]
fn a_zero_reading_is_allowed() {
    assert!(Meter::new(Some(0.0), MeterType::Kilometer).is_ok());
}

#[test]
fn a_negative_reading_is_refused() {
    assert_eq!(
        Meter::new(Some(-1.0), MeterType::Kilometer),
        Err(QuoteError::InvalidMeterReading { reading: -1.0 })
    );
}

#[test]
fn a_reading_that_is_not_a_number_is_refused() {
    let result = Meter::new(Some(f64::NAN), MeterType::Hour);
    assert!(matches!(
        result,
        Err(QuoteError::InvalidMeterReading { .. })
    ));
}

// ===== equipment source and drafts =====

fn corolla_identification() -> Identification {
    Identification::new(Some(String::from("SERIAL_NUMBER_TEST_1")), None).unwrap()
}

fn corolla_meter() -> Meter {
    Meter::new(Some(28156.98), MeterType::Kilometer).unwrap()
}

fn corolla_draft() -> EquipmentDraft {
    EquipmentDraft::new(
        "test catalog",
        String::from("Toyota"),
        String::from("Corolla Cross"),
        Some(2025),
        EquipmentType::Car,
    )
}

#[test]
fn equipment_typed_in_by_hand_says_so() {
    assert_eq!(corolla().source(), EquipmentSource::Manual);
    assert_eq!(corolla().source().to_string(), "entered by hand");
}

#[test]
fn equipment_from_a_draft_remembers_which_lookup_supplied_it() {
    let equipment =
        Equipment::from_draft(corolla_draft(), corolla_identification(), corolla_meter()).unwrap();
    assert_eq!(equipment.source(), EquipmentSource::Lookup("test catalog"));
    assert_eq!(
        equipment.source().to_string(),
        "looked up with test catalog"
    );
}

#[test]
fn equipment_from_a_draft_shows_the_same_details_as_equipment_typed_in() {
    let from_draft =
        Equipment::from_draft(corolla_draft(), corolla_identification(), corolla_meter()).unwrap();
    let by_hand = Equipment::new(
        String::from("Toyota"),
        String::from("Corolla Cross"),
        Some(2025),
        corolla_identification(),
        corolla_meter(),
        EquipmentType::Car,
    )
    .unwrap();
    assert_eq!(from_draft.to_string(), by_hand.to_string());
    assert_eq!(from_draft.equipment_type(), EquipmentType::Car);
}

#[test]
fn a_draft_with_a_blank_make_is_refused() {
    let draft = EquipmentDraft::new(
        "test catalog",
        String::from("  "),
        String::from("Corolla Cross"),
        None,
        EquipmentType::Car,
    );
    assert_eq!(
        Equipment::from_draft(draft, corolla_identification(), corolla_meter()),
        Err(QuoteError::MissingField { field: "make" })
    );
}

#[test]
fn a_draft_with_a_blank_model_is_refused() {
    let draft = EquipmentDraft::new(
        "test catalog",
        String::from("Toyota"),
        String::new(),
        None,
        EquipmentType::Car,
    );
    assert_eq!(
        Equipment::from_draft(draft, corolla_identification(), corolla_meter()),
        Err(QuoteError::MissingField { field: "model" })
    );
}

// ===== id (given by the database, never made by Quoto) =====

fn some_id() -> uuid::Uuid {
    uuid::Uuid::from_u128(0x3333)
}

#[test]
fn new_equipment_has_no_id() {
    assert_eq!(corolla().id(), None);
}

#[test]
fn equipment_built_from_a_lookup_has_no_id_either() {
    let draft = samples::equipment_lookup()
        .lookup(samples::COROLLA_SERIAL)
        .unwrap()
        .unwrap();
    assert_eq!(samples::corolla_from(draft).unwrap().id(), None);
}

#[test]
fn equipment_loaded_from_the_database_has_its_id_and_keeps_its_source() {
    let draft = samples::equipment_lookup()
        .lookup(samples::COROLLA_SERIAL)
        .unwrap()
        .unwrap();
    let looked_up = samples::corolla_from(draft).unwrap().with_id(some_id());
    assert_eq!(looked_up.id(), Some(some_id()));
    assert_eq!(
        looked_up.source().to_string(),
        "looked up with demo catalog"
    );
}

#[test]
fn giving_equipment_an_id_does_not_change_what_it_shows() {
    let with = corolla().with_id(some_id());
    assert_eq!(with.to_string(), corolla().to_string());
    assert!(!with.to_string().contains(&some_id().to_string()));
}

#[test]
fn equipment_with_the_same_details_is_the_same_only_if_the_id_matches() {
    let a = corolla().with_id(some_id());
    assert_eq!(a, corolla().with_id(some_id()));
    assert_ne!(a, corolla().with_id(uuid::Uuid::from_u128(0x4444)));
    assert_ne!(a, corolla());
}

// ===== owner (the customer's id, given when the machine is loaded from the database) =====

#[test]
fn a_new_machine_has_no_owner_id() {
    assert_eq!(corolla().customer_id(), None);
}

#[test]
fn a_machine_loaded_from_the_database_knows_its_owner() {
    let owner = uuid::Uuid::from_u128(0xAAA);
    let machine = corolla().with_customer_id(owner);
    assert_eq!(machine.customer_id(), Some(owner));
    assert_eq!(machine.id(), None); // the owner is not the machine's own id
}

#[test]
fn the_owner_id_is_never_printed() {
    let owner = uuid::Uuid::from_u128(0xAAA);
    assert_eq!(
        corolla().with_customer_id(owner).to_string(),
        corolla().to_string()
    );
}
