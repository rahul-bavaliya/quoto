// tests/branch.rs: tests for the branch feature (public API only)

use quoto::*;

fn address(province: Province) -> Address {
    Address::new(
        String::from("1 Industrial Rd"),
        None,
        String::from("Regina"),
        PostalCode::parse("S4P 3Y2").unwrap(),
        province,
    )
    .unwrap()
}

fn phone() -> PhoneNumber {
    PhoneNumber::parse("306-555-0100").unwrap()
}

fn branch(code: &str, name: &str) -> QuoteResult<Branch> {
    Branch::new(
        String::from(code),
        String::from(name),
        address(Province::Saskatchewan),
        phone(),
    )
}

#[test]
fn a_branch_is_built_from_a_code_a_name_an_address_and_a_phone() {
    let branch = branch("REG", "Regina Branch").unwrap();
    assert_eq!(branch.code(), "REG");
    assert_eq!(branch.name(), "Regina Branch");
    assert_eq!(branch.phone(), &phone());
}

#[test]
fn a_blank_code_is_refused() {
    assert!(matches!(
        branch("   ", "Regina Branch"),
        Err(QuoteError::MissingField {
            field: "branch code"
        })
    ));
}

#[test]
fn a_blank_name_is_refused() {
    assert!(matches!(
        branch("REG", ""),
        Err(QuoteError::MissingField {
            field: "branch name"
        })
    ));
}

#[test]
fn the_branch_province_comes_from_its_address() {
    let alberta = Branch::new(
        String::from("CGY"),
        String::from("Calgary Branch"),
        address(Province::Alberta),
        phone(),
    )
    .unwrap();
    assert_eq!(alberta.province(), &Province::Alberta);
}

#[test]
fn a_new_branch_has_no_id() {
    assert_eq!(branch("REG", "Regina Branch").unwrap().id(), None);
}

#[test]
fn a_branch_loaded_from_the_database_has_its_id() {
    let id = uuid::Uuid::from_u128(0x4444);
    let branch = branch("REG", "Regina Branch").unwrap().with_id(id);
    assert_eq!(branch.id(), Some(id));
}

#[test]
fn the_sample_branches_are_in_their_provinces() {
    assert_eq!(
        samples::branch_in(Province::Saskatchewan)
            .unwrap()
            .province(),
        &Province::Saskatchewan
    );
    assert_eq!(
        samples::branch_in(Province::Alberta).unwrap().province(),
        &Province::Alberta
    );
}
