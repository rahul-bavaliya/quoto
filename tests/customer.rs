// tests/customer.rs: tests for the customer feature (public API only)

use quoto::*;

// ===== customer =====

fn sample_contact() -> Contact {
    Contact::new(
        PhoneNumber::parse("306-555-0142").unwrap(),
        None,
        Some(Email::parse("jane@prairiefarms.ca").unwrap()),
    )
}

fn sample_address() -> Address {
    Address::new(
        String::from("123 Main St"),
        None,
        String::from("Regina"),
        PostalCode::parse("S4P 3Y2").unwrap(),
        Province::Saskatchewan,
    )
    .unwrap()
}

#[test]
fn customer_with_company_and_title_shows_both() {
    let customer = Customer::new(
        String::from("Jane Smith"),
        Some(String::from("Prairie Farms Ltd.")),
        Some(String::from("Service Manager")),
        sample_contact(),
        sample_address(),
    )
    .unwrap();
    assert_eq!(
        customer.to_string(),
        "Jane Smith (Service Manager, Prairie Farms Ltd.)"
    );
}

#[test]
fn customer_without_company_shows_just_the_name() {
    let customer = Customer::new(
        String::from("Bob Lee"),
        None,
        None,
        sample_contact(),
        sample_address(),
    )
    .unwrap();
    assert_eq!(customer.to_string(), "Bob Lee");
}

#[test]
fn customer_without_name_is_refused() {
    let result = Customer::new(
        String::from(""),
        None,
        None,
        sample_contact(),
        sample_address(),
    );
    assert_eq!(
        result,
        Err(QuoteError::MissingField {
            field: "customer name"
        })
    );
}

#[test]
fn customer_province_comes_from_the_address() {
    let customer = Customer::new(
        String::from("Jane Smith"),
        None,
        None,
        sample_contact(),
        sample_address(),
    )
    .unwrap();
    assert_eq!(customer.province(), &Province::Saskatchewan);
}

// ===== address =====

fn regina_postal_code() -> PostalCode {
    PostalCode::parse("S4P 3Y2").unwrap()
}

#[test]
fn address_shows_on_one_line() {
    let address = Address::new(
        String::from("123 Main St"),
        Some(String::from("Unit 4")),
        String::from("Regina"),
        regina_postal_code(),
        Province::Saskatchewan,
    )
    .unwrap();
    assert_eq!(
        address.to_string(),
        "123 Main St, Unit 4, Regina, SK S4P 3Y2"
    );
}

#[test]
fn address_without_line1_is_refused() {
    let result = Address::new(
        String::from("  "),
        None,
        String::from("Regina"),
        regina_postal_code(),
        Province::Saskatchewan,
    );
    assert_eq!(
        result,
        Err(QuoteError::MissingField {
            field: "address line 1"
        })
    );
}

#[test]
fn address_without_city_is_refused() {
    let result = Address::new(
        String::from("123 Main St"),
        None,
        String::from(""),
        regina_postal_code(),
        Province::Saskatchewan,
    );
    assert_eq!(result, Err(QuoteError::MissingField { field: "city" }));
}

// ===== postal code =====

#[test]
fn postal_code_is_cleaned_up() {
    let code = PostalCode::parse(" s4p-3y2 ").unwrap();
    assert_eq!(code.to_string(), "S4P 3Y2");
}

#[test]
fn postal_code_with_wrong_shape_is_refused() {
    let result = PostalCode::parse("12345");
    assert!(matches!(result, Err(QuoteError::InvalidPostalCode { .. })));
}

#[test]
fn postal_code_too_short_is_refused() {
    assert!(PostalCode::parse("S4P 3Y").is_err());
}

// ===== contact =====

#[test]
fn contact_with_only_a_phone_prints_just_the_phone() {
    let phone = PhoneNumber::parse("306-555-0142").unwrap();
    let contact = Contact::new(phone, None, None);
    assert_eq!(contact.to_string(), "+1-306-555-0142");
}

#[test]
fn contact_with_phone_and_email_prints_both_on_one_line() {
    let phone = PhoneNumber::parse("306-555-0142").unwrap();
    let email = Email::parse("Rahul@test.com").unwrap();
    let contact = Contact::new(phone, None, Some(email));
    assert_eq!(
        contact.to_string(),
        "+1-306-555-0142 · Email: rahul@test.com"
    );
}

#[test]
fn contact_with_everything_prints_phone_alternate_and_email() {
    let phone = PhoneNumber::parse("306-555-0142").unwrap();
    let alternate_phone = PhoneNumber::parse("306-555-1234").unwrap();
    let email = Email::parse("Rahul@test.com").unwrap();
    let contact = Contact::new(phone, Some(alternate_phone), Some(email));
    assert_eq!(
        contact.to_string(),
        "+1-306-555-0142 · Alternate Phone: +1-306-555-1234 · Email: rahul@test.com"
    );
}

// ===== phone number =====

#[test]
fn phone_with_dashes_is_accepted() {
    let phone = PhoneNumber::parse("306-555-0142").unwrap();
    assert_eq!(phone.to_string(), "+1-306-555-0142");
}

#[test]
fn phone_with_brackets_and_spaces_is_cleaned_up() {
    let phone = PhoneNumber::parse("(306) 555 0142").unwrap();
    assert_eq!(phone.to_string(), "+1-306-555-0142");
}

#[test]
fn phone_with_country_code_is_accepted() {
    let phone = PhoneNumber::parse("+1 306 555 0142").unwrap();
    assert_eq!(phone.to_string(), "+1-306-555-0142");
}

#[test]
fn short_phone_is_refused() {
    let result = PhoneNumber::parse("555-0142");
    assert!(matches!(result, Err(QuoteError::InvalidPhone { .. })));
}

#[test]
fn phone_with_letters_is_refused() {
    let result = PhoneNumber::parse("306-555-01AB");
    assert!(matches!(result, Err(QuoteError::InvalidPhone { .. })));
}

// ===== email =====

#[test]
fn normal_email_is_accepted() {
    assert!(Email::parse("rahul@example.ca").is_ok());
}

#[test]
fn email_is_stored_in_lowercase() {
    let email = Email::parse("Rahul@Example.CA").unwrap();
    assert_eq!(email.to_string(), "rahul@example.ca");
}

#[test]
fn email_without_at_is_refused() {
    assert!(Email::parse("rahul.example.ca").is_err());
}

#[test]
fn email_without_dot_in_domain_is_refused() {
    assert!(Email::parse("rahul@example").is_err());
}

#[test]
fn email_with_space_is_refused() {
    assert!(Email::parse("ra hul@example.ca").is_err());
}

#[test]
fn email_with_nothing_before_at_is_refused() {
    assert!(Email::parse("@example.ca").is_err());
}

#[test]
fn email_with_two_ats_is_refused() {
    assert!(Email::parse("rahul@@example.ca").is_err());
}
