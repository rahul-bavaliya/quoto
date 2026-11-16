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
fn equipment_sample_test() {
    assert_eq!(corolla().make(), "Toyota");
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
