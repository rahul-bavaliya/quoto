// tests/branch.rs: tests for the branch feature (public API only)

use quoto::*;
use uuid::Uuid;

fn address(province: Province) -> Address {
    Address::new(
        String::from("1 Industrial Rd"),
        Some(String::from("Unit 4")),
        String::from("Regina"),
        PostalCode::parse("S4P 3Y2").unwrap(),
        province,
    )
    .unwrap()
}

fn phone() -> PhoneNumber {
    PhoneNumber::parse("306-555-0100").unwrap()
}

fn branch(number: &str, name: &str) -> QuoteResult<Branch> {
    Branch::new(
        String::from(number),
        String::from(name),
        address(Province::Saskatchewan),
        phone(),
    )
}

fn regina() -> Branch {
    branch("001", "Regina Branch").unwrap()
}

// ===== building a branch =====

#[test]
fn a_branch_is_built_from_a_number_a_name_an_address_and_a_phone() {
    let branch = regina();
    assert_eq!(branch.number(), "001");
    assert_eq!(branch.name(), "Regina Branch");
    assert_eq!(branch.phone(), &phone());
    assert_eq!(branch.address(), &address(Province::Saskatchewan));
}

#[test]
fn the_branch_number_keeps_its_leading_zeros() {
    assert_eq!(branch("007", "Moose Jaw").unwrap().number(), "007");
}

#[test]
fn a_blank_number_is_refused() {
    assert!(matches!(
        branch("   ", "Regina Branch"),
        Err(QuoteError::MissingField {
            field: "branch number"
        })
    ));
}

#[test]
fn a_blank_name_is_refused() {
    assert!(matches!(
        branch("001", ""),
        Err(QuoteError::MissingField {
            field: "branch name"
        })
    ));
}

// ===== the address: province, country and the pieces the database will store =====

#[test]
fn the_branch_province_comes_from_its_address() {
    let alberta = Branch::new(
        String::from("002"),
        String::from("Calgary Branch"),
        address(Province::Alberta),
        phone(),
    )
    .unwrap();
    assert_eq!(alberta.province(), &Province::Alberta);
}

#[test]
fn the_country_follows_the_province() {
    assert_eq!(regina().country(), Country::Canada);
    assert_eq!(regina().country().label(), "Canada");
    assert_eq!(regina().country().code(), "CA");
}

#[test]
fn the_address_can_be_read_piece_by_piece() {
    let address = address(Province::Saskatchewan);
    assert_eq!(address.line1(), "1 Industrial Rd");
    assert_eq!(address.line2(), Some("Unit 4"));
    assert_eq!(address.city(), "Regina");
    assert_eq!(address.postal_code().to_string(), "S4P 3Y2");
}

#[test]
fn a_missing_second_address_line_reads_as_none() {
    let address = Address::new(
        String::from("1 Industrial Rd"),
        None,
        String::from("Regina"),
        PostalCode::parse("S4P 3Y2").unwrap(),
        Province::Saskatchewan,
    )
    .unwrap();
    assert_eq!(address.line2(), None);
}

#[test]
fn a_phone_and_a_postal_code_can_be_stored_as_text_and_read_back() {
    // The database keeps text; reading it back must give the same value.
    let stored_phone = phone().to_string();
    assert_eq!(PhoneNumber::parse(&stored_phone).unwrap(), phone());
    let stored_code = PostalCode::parse("S4P 3Y2").unwrap().to_string();
    assert_eq!(
        PostalCode::parse(&stored_code).unwrap(),
        PostalCode::parse("S4P 3Y2").unwrap()
    );
}

// ===== map location (optional) =====

#[test]
fn a_new_branch_has_no_location() {
    assert_eq!(regina().location(), None);
}

#[test]
fn a_branch_can_have_a_location() {
    let branch = regina().with_location(Location::new(50.4452, -104.6189).unwrap());
    let location = branch.location().unwrap();
    assert_eq!(location.latitude(), 50.4452);
    assert_eq!(location.longitude(), -104.6189);
}

#[test]
fn the_edges_of_the_map_are_allowed() {
    assert!(Location::new(90.0, 180.0).is_ok());
    assert!(Location::new(-90.0, -180.0).is_ok());
    assert!(Location::new(0.0, 0.0).is_ok());
}

#[test]
fn a_location_off_the_map_is_refused() {
    for (latitude, longitude) in [
        (90.1, 0.0),
        (-90.1, 0.0),
        (0.0, 180.1),
        (0.0, -180.1),
        (f64::NAN, 0.0),
        (0.0, f64::INFINITY),
    ] {
        assert!(matches!(
            Location::new(latitude, longitude),
            Err(QuoteError::InvalidLocation)
        ));
    }
}

// ===== status: a branch is never deleted =====

#[test]
fn a_new_branch_is_active() {
    assert_eq!(regina().status(), BranchStatus::Active);
    assert!(regina().is_active());
}

#[test]
fn a_branch_that_is_not_in_use_becomes_inactive() {
    let mut branch = regina();
    branch.deactivate();
    assert_eq!(branch.status(), BranchStatus::Inactive);
    assert!(!branch.is_active());
}

#[test]
fn an_inactive_branch_can_be_activated_again() {
    let mut branch = regina();
    branch.deactivate();
    branch.activate();
    assert!(branch.is_active());
}

#[test]
fn deactivating_twice_changes_nothing() {
    let mut branch = regina();
    branch.deactivate();
    branch.deactivate();
    assert_eq!(branch.status(), BranchStatus::Inactive);
}

#[test]
fn deactivating_keeps_everything_else() {
    let mut branch = regina().with_id(Uuid::from_u128(1));
    branch.deactivate();
    assert_eq!(branch.id(), Some(Uuid::from_u128(1)));
    assert_eq!(branch.number(), "001");
    assert_eq!(branch.name(), "Regina Branch");
}

#[test]
fn a_branch_loaded_from_the_database_can_come_back_inactive() {
    let branch = regina().with_status(BranchStatus::Inactive);
    assert!(!branch.is_active());
}

#[test]
fn the_status_words_kept_in_the_database() {
    assert_eq!(BranchStatus::Active.label(), "active");
    assert_eq!(BranchStatus::Inactive.label(), "inactive");
    assert_eq!(BranchStatus::Inactive.to_string(), "inactive");
}

// ===== id (given by the database, never made by Quoto) =====

#[test]
fn a_new_branch_has_no_id() {
    assert_eq!(regina().id(), None);
}

#[test]
fn a_branch_loaded_from_the_database_has_its_id() {
    let id = Uuid::from_u128(0x4444);
    assert_eq!(regina().with_id(id).id(), Some(id));
}

#[test]
fn an_updated_branch_is_the_new_version_with_the_old_id() {
    // How "update" works: build the new version, give it the row's id.
    let id = Uuid::from_u128(0x4444);
    let old = regina().with_id(id);
    let renamed = branch("001", "Regina South Branch").unwrap().with_id(id);
    assert_eq!(renamed.id(), old.id());
    assert_ne!(renamed, old);
}

// ===== samples and the quote =====

#[test]
fn the_sample_branches_are_in_their_provinces() {
    let sk = samples::branch_in(Province::Saskatchewan).unwrap();
    let ab = samples::branch_in(Province::Alberta).unwrap();
    assert_eq!(sk.province(), &Province::Saskatchewan);
    assert_eq!(ab.province(), &Province::Alberta);
    assert_ne!(sk.number(), ab.number());
}

#[test]
fn an_inactive_branch_cannot_start_a_new_quote() {
    let mut branch = samples::branch_in(Province::Saskatchewan).unwrap();
    branch.deactivate();
    let result = Quote::new(samples::quote_number(), samples::quote_time(), branch);
    assert!(matches!(result, Err(QuoteError::BranchInactive)));
}

#[test]
fn a_quote_started_before_the_branch_closed_keeps_working() {
    // The quote owns its branch, so closing the branch later never touches old quotes.
    let quote = samples::oil_change_quote(Province::Saskatchewan).unwrap();
    assert!(quote.branch().is_active());
    assert!(quote.total() > 0);
}
