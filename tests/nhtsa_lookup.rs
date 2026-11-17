// tests/nhtsa_lookup.rs: tests for the NHTSA lookup (public API only, never the internet)
//
// The real service is replaced by `NhtsaLookup::with_fetcher`, which hands back
// a saved answer shaped like NHTSA's. 1HGCM82633A004352 is the example VIN from
// NHTSA's own documentation: a valid 17-character VIN, not a customer's machine.

use quoto::*;
use serde_json::{Value, json};
use std::cell::RefCell;
use std::rc::Rc;

const VIN: &str = "1HGCM82633A004352";

// ===== helpers =====

/// An answer like NHTSA's: HTTP 200 and a flat row of text values.
fn answer(error_code: &str, make: &str, model: &str, year: &str, vehicle_type: &str) -> String {
    json!({
        "Count": 1,
        "Message": "Results returned successfully.",
        "Results": [{
            "ErrorCode": error_code,
            "Make": make,
            "Model": model,
            "ModelYear": year,
            "VehicleType": vehicle_type,
            "BodyClass": "Pickup"
        }]
    })
    .to_string()
}

/// A lookup that always gets this answer.
fn lookup_answering(text: String) -> NhtsaLookup {
    NhtsaLookup::with_fetcher(move |_url| Ok(text.clone()))
}

/// A lookup that remembers every web address it was asked for.
fn lookup_recording_requests(text: String) -> (NhtsaLookup, Rc<RefCell<Vec<String>>>) {
    let requests = Rc::new(RefCell::new(Vec::new()));
    let seen = Rc::clone(&requests);
    let lookup = NhtsaLookup::with_fetcher(move |url| {
        seen.borrow_mut().push(url.to_string());
        Ok(text.clone())
    });
    (lookup, requests)
}

fn expected(make: &str, model: &str, year: Option<i64>, kind: EquipmentType) -> EquipmentDraft {
    EquipmentDraft::new("NHTSA vPIC", make.into(), model.into(), year, kind)
}

fn failed() -> QuoteError {
    QuoteError::LookupFailed {
        provider: "NHTSA vPIC",
    }
}

// ===== reading a good answer =====

#[test]
fn a_clean_answer_becomes_a_draft() {
    let lookup = lookup_answering(answer("0", "HONDA", "Accord", "2003", "PASSENGER CAR"));
    assert_eq!(
        lookup.lookup(VIN),
        Ok(Some(expected(
            "HONDA",
            "Accord",
            Some(2003),
            EquipmentType::Car
        )))
    );
}

#[test]
fn the_service_is_asked_for_the_vin_in_the_right_address() {
    let (lookup, requests) =
        lookup_recording_requests(answer("0", "HONDA", "Accord", "2003", "PASSENGER CAR"));
    lookup.lookup(VIN).unwrap();
    assert_eq!(
        *requests.borrow(),
        vec![format!(
            "https://vpic.nhtsa.dot.gov/api/vehicles/DecodeVinValues/{VIN}?format=json"
        )]
    );
}

#[test]
fn the_vin_is_cleaned_up_before_it_is_sent() {
    let (lookup, requests) =
        lookup_recording_requests(answer("0", "HONDA", "Accord", "2003", "PASSENGER CAR"));
    lookup.lookup("  1hgcm82633a004352 ").unwrap();
    assert!(requests.borrow()[0].contains(&format!("/{VIN}?")));
}

#[test]
fn spaces_around_the_make_and_model_are_trimmed() {
    let lookup = lookup_answering(answer("0", " HONDA ", " Accord ", "2003", "PASSENGER CAR"));
    assert_eq!(
        lookup.lookup(VIN),
        Ok(Some(expected(
            "HONDA",
            "Accord",
            Some(2003),
            EquipmentType::Car
        )))
    );
}

// ===== what kind of machine is it =====

fn assert_kind(vehicle_type: &str, kind: EquipmentType) {
    let lookup = lookup_answering(answer("0", "MAKE", "Model", "2020", vehicle_type));
    assert_eq!(
        lookup.lookup(VIN),
        Ok(Some(expected("MAKE", "Model", Some(2020), kind))),
        "vehicle type {vehicle_type:?}"
    );
}

#[test]
fn the_vehicle_type_decides_the_kind_of_machine() {
    assert_kind("PASSENGER CAR", EquipmentType::Car);
    assert_kind("MULTIPURPOSE PASSENGER VEHICLE (MPV)", EquipmentType::Car);
    assert_kind("TRUCK", EquipmentType::Truck);
    assert_kind("truck", EquipmentType::Truck);
    assert_kind("TRAILER", EquipmentType::Trailer);
}

#[test]
fn an_unclear_vehicle_type_is_other() {
    assert_kind("BUS", EquipmentType::Other);
    assert_kind("INCOMPLETE VEHICLE", EquipmentType::Other);
    assert_kind("", EquipmentType::Other);
}

// ===== answers we do not trust or cannot use =====

#[test]
fn an_answer_with_an_error_code_is_not_trusted() {
    // 1 = the VIN's check digit is wrong (probably a typo); NHTSA still answers 200.
    for code in ["1", "8", "11", "6,14", ""] {
        let lookup = lookup_answering(answer(code, "HONDA", "Accord", "2003", "PASSENGER CAR"));
        assert_eq!(lookup.lookup(VIN), Ok(None), "error code {code:?}");
    }
}

#[test]
fn an_answer_without_a_make_or_a_model_is_not_found() {
    let no_make = lookup_answering(answer("0", "", "Accord", "2003", "PASSENGER CAR"));
    let no_model = lookup_answering(answer("0", "HONDA", "  ", "2003", "PASSENGER CAR"));
    assert_eq!(no_make.lookup(VIN), Ok(None));
    assert_eq!(no_model.lookup(VIN), Ok(None));
}

#[test]
fn a_missing_year_is_fine() {
    for year in ["", "unknown"] {
        let lookup = lookup_answering(answer("0", "HONDA", "Accord", year, "PASSENGER CAR"));
        assert_eq!(
            lookup.lookup(VIN),
            Ok(Some(expected("HONDA", "Accord", None, EquipmentType::Car)))
        );
    }
}

#[test]
fn fields_that_are_null_or_missing_are_read_as_blank() {
    let nulls = json!({"Results": [{"ErrorCode": "0", "Make": null, "Model": "Accord"}]});
    let missing = json!({"Results": [{"ErrorCode": "0", "Make": "HONDA"}]});
    assert_eq!(lookup_answering(nulls.to_string()).lookup(VIN), Ok(None));
    assert_eq!(lookup_answering(missing.to_string()).lookup(VIN), Ok(None));

    let year_and_type_missing =
        json!({"Results": [{"ErrorCode": "0", "Make": "HONDA", "Model": "Accord"}]});
    assert_eq!(
        lookup_answering(year_and_type_missing.to_string()).lookup(VIN),
        Ok(Some(expected(
            "HONDA",
            "Accord",
            None,
            EquipmentType::Other
        )))
    );
}

// ===== serial numbers that are not VINs never reach the service =====

#[test]
fn a_serial_that_is_not_a_vin_is_not_sent_to_the_service() {
    let not_vins = [
        "",
        "   ",
        "DEMO-8R410-0001",
        "SERIAL_NUMBER_TEST_1",
        "1HGCM82633A00435",   // 16 characters
        "1HGCM82633A0043521", // 18 characters
        "1HGCM82633A00435I",  // I is never used in a VIN
        "1HGCM82633A00435O",  // neither is O
        "1HGCM82633A00435Q",  // nor Q
        "../../etc/passwd/xx",
        "1HGCM82633A00435/",
        "1HGCM82633A00435?",
    ];
    for serial in not_vins {
        let (lookup, requests) = lookup_recording_requests(String::new());
        assert_eq!(lookup.lookup(serial), Ok(None), "serial {serial:?}");
        assert!(requests.borrow().is_empty(), "serial {serial:?} was sent");
    }
}

// ===== when the service cannot be used =====

#[test]
fn a_failed_request_is_reported_as_a_failed_lookup() {
    let lookup = NhtsaLookup::with_fetcher(|_url| Err(failed()));
    assert_eq!(lookup.lookup(VIN), Err(failed()));
}

#[test]
fn an_answer_that_is_not_json_is_a_failed_lookup() {
    for text in ["", "<html>Service busy</html>", "not json at all"] {
        assert_eq!(
            lookup_answering(text.to_string()).lookup(VIN),
            Err(failed()),
            "text {text:?}"
        );
    }
}

#[test]
fn an_answer_without_results_is_a_failed_lookup() {
    let empty_results = json!({"Count": 0, "Results": []});
    let wrong_shape: Value = json!({"Message": "hello"});
    assert_eq!(
        lookup_answering(empty_results.to_string()).lookup(VIN),
        Err(failed())
    );
    assert_eq!(
        lookup_answering(wrong_shape.to_string()).lookup(VIN),
        Err(failed())
    );
}

// ===== in a chain with other lookups =====

#[test]
fn a_chain_uses_nhtsa_for_vins_and_the_catalog_for_other_serials() {
    let (nhtsa, requests) =
        lookup_recording_requests(answer("0", "HONDA", "Accord", "2003", "PASSENGER CAR"));
    let chain = ChainedLookup::new(vec![Box::new(nhtsa), Box::new(samples::equipment_lookup())]);

    // A road vehicle: NHTSA answers.
    assert_eq!(
        chain.lookup(VIN),
        Ok(Some(expected(
            "HONDA",
            "Accord",
            Some(2003),
            EquipmentType::Car
        )))
    );
    // A tractor's serial is not a VIN: NHTSA is not even asked, the catalog answers.
    let tractor = chain.lookup(samples::TRACTOR_SERIAL).unwrap().unwrap();
    assert_eq!(
        Equipment::from_draft(
            tractor,
            Identification::new(Some(samples::TRACTOR_SERIAL.into()), None).unwrap(),
            Meter::new(Some(1250.5), MeterType::Hour).unwrap(),
        )
        .unwrap()
        .equipment_type(),
        EquipmentType::Tractor
    );
    assert_eq!(requests.borrow().len(), 1);
}

#[test]
fn a_chain_falls_back_to_the_catalog_when_nhtsa_is_down() {
    let down = NhtsaLookup::with_fetcher(|_url| Err(failed()));
    let mut catalog = InMemoryEquipmentLookup::new();
    catalog.add(
        VIN,
        EquipmentDraft::new(
            "demo catalog",
            "Honda".into(),
            "Accord".into(),
            Some(2003),
            EquipmentType::Car,
        ),
    );
    let chain = ChainedLookup::new(vec![Box::new(down), Box::new(catalog)]);
    assert!(chain.lookup(VIN).unwrap().is_some());
}

#[test]
fn a_chain_tells_the_person_when_nhtsa_is_down_and_nobody_else_knows() {
    let down = NhtsaLookup::with_fetcher(|_url| Err(failed()));
    let chain = ChainedLookup::new(vec![Box::new(down), Box::new(samples::equipment_lookup())]);
    assert_eq!(chain.lookup(VIN), Err(failed()));
}
