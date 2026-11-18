// src/samples.rs: made-up sample data, in ONE place
//
// The tests and the examples all
// need the same pretend customers, machines and quote lines. They live here
// so a change (like a new `Equipment::new` input) is made once, not three times.
//
// Everything here is invented: no real people, companies or phone numbers.

use crate::branch::Branch;
use crate::customer::{Address, Contact, Customer, Email, PhoneNumber, PostalCode};
use crate::equipment::{
    Equipment, EquipmentDraft, EquipmentType, Identification, Meter, MeterType,
};
use crate::equipment_lookup::InMemoryEquipmentLookup;
use crate::errors::QuoteResult;
use crate::province::Province;
use crate::quote::{InMemoryQuoteNumbers, LineItem, LineKind, Quote, QuoteNumber};
use chrono::{NaiveDate, NaiveDateTime};

/// A branch in the given province (that decides the taxes).
pub fn branch_in(province: Province) -> QuoteResult<Branch> {
    let (number, name, city, postal_code, phone) = match province {
        Province::Saskatchewan => ("001", "Regina Branch", "Regina", "S4P 3Y2", "306-555-0100"),
        Province::Alberta => (
            "002",
            "Calgary Branch",
            "Calgary",
            "T2P 1J9",
            "403-555-0100",
        ),
    };
    let address = Address::new(
        String::from("1 Industrial Rd"),
        None,
        String::from(city),
        PostalCode::parse(postal_code)?,
        province,
    )?;
    Branch::new(
        String::from(number),
        String::from(name),
        address,
        PhoneNumber::parse(phone)?,
    )
}

/// A plain customer who lives in the given province.
pub fn customer_in(province: Province) -> QuoteResult<Customer> {
    let (city, postal_code) = match province {
        Province::Saskatchewan => ("Regina", "S4P 3Y2"),
        Province::Alberta => ("Calgary", "T2P 1J9"),
    };
    let contact = Contact::new(PhoneNumber::parse("306-555-0142")?, None, None);
    let address = Address::new(
        String::from("123 Main St"),
        None,
        String::from(city),
        PostalCode::parse(postal_code)?,
        province,
    )?;
    Customer::new(String::from("Jane Smith"), None, None, contact, address)
}

/// A Saskatchewan customer with a company, a title and an email: GST + PST.
pub fn prairie_farms() -> QuoteResult<Customer> {
    let contact = Contact::new(
        PhoneNumber::parse("306-555-0142")?,
        None,
        Some(Email::parse("jane@prairiefarms.ca")?),
    );
    let address = Address::new(
        String::from("123 Main St"),
        None,
        String::from("Regina"),
        PostalCode::parse("S4P 3Y2")?,
        Province::Saskatchewan,
    )?;
    Customer::new(
        String::from("Jane Smith"),
        Some(String::from("Prairie Farms Ltd.")),
        Some(String::from("Service Manager")),
        contact,
        address,
    )
}

/// An Alberta customer with an alternate phone: GST only, no PST.
pub fn northern_oilfield() -> QuoteResult<Customer> {
    let contact = Contact::new(
        PhoneNumber::parse("780-555-0188")?,
        Some(PhoneNumber::parse("306-555-8965")?),
        Some(Email::parse("mike@northernoilfield.ca")?),
    );
    let address = Address::new(
        String::from("456 Jasper Ave"),
        None,
        String::from("Edmonton"),
        PostalCode::parse("T5J 0N3")?,
        Province::Alberta,
    )?;
    Customer::new(
        String::from("Mike Brown"),
        Some(String::from("Northern Oilfield Services")),
        Some(String::from("Fleet Supervisor")),
        contact,
        address,
    )
}

/// A car. A fresh one each time: a quote owns its equipment, so two quotes
/// can't share one.
/// The serial number of the sample Corolla (the demo catalog knows it).
pub const COROLLA_SERIAL: &str = "SERIAL_NUMBER_TEST_1";

/// The serial number of the sample tractor (the demo catalog knows it).
pub const TRACTOR_SERIAL: &str = "DEMO-8R410-0001";

/// The name the demo catalog gives itself as a source.
const DEMO_CATALOG: &str = "demo catalog";

fn corolla_identification() -> QuoteResult<Identification> {
    Identification::new(
        Some(String::from(COROLLA_SERIAL)),
        Some(String::from("RB97")),
    )
}

fn corolla_meter() -> QuoteResult<Meter> {
    Meter::new(Some(28156.98), MeterType::Kilometer)
}

/// The Corolla, typed in by hand.
pub fn corolla() -> QuoteResult<Equipment> {
    Equipment::new(
        String::from("Toyota"),
        String::from("Corolla Cross"),
        Some(2025),
        corolla_identification()?,
        corolla_meter()?,
        EquipmentType::Car,
    )
}

/// The Corolla built from a lookup's answer: the writer adds the plate and the meter.
pub fn corolla_from(draft: EquipmentDraft) -> QuoteResult<Equipment> {
    Equipment::from_draft(draft, corolla_identification()?, corolla_meter()?)
}

/// A demo catalog that knows the Corolla and the tractor, and nothing else.
pub fn equipment_lookup() -> InMemoryEquipmentLookup {
    let mut lookup = InMemoryEquipmentLookup::new();
    lookup.add(
        COROLLA_SERIAL,
        EquipmentDraft::new(
            DEMO_CATALOG,
            String::from("Toyota"),
            String::from("Corolla Cross"),
            Some(2025),
            EquipmentType::Car,
        ),
    );
    lookup.add(
        TRACTOR_SERIAL,
        EquipmentDraft::new(
            DEMO_CATALOG,
            String::from("John Deere"),
            String::from("8R 410"),
            Some(2023),
            EquipmentType::Tractor,
        ),
    );
    lookup
}

/// An oilfield service truck.
pub fn oilfield_truck() -> QuoteResult<Equipment> {
    Equipment::new(
        String::from("Ford"),
        String::from("F-350 Super Duty"),
        Some(2022),
        Identification::new(
            Some(String::from("1FT8W3BT0NEC12345")),
            Some(String::from("BXK 4471")),
        )?,
        Meter::new(Some(143250.0), MeterType::Kilometer)?,
        EquipmentType::Truck,
    )
}

/// The standard 5-line oil-change job ($198.00 before tax). No credits.
/// Fresh lines each time, because each quote must own its own lines.
pub fn oil_change_lines() -> QuoteResult<Vec<LineItem>> {
    let lines = [
        ("Oil Filter", 1.0, 2500, LineKind::Part),
        ("Oil Change Labour", 0.5, 16000, LineKind::Labour),
        ("Engine Oil", 6.0, 850, LineKind::Part),
        ("Shop Supplies", 1.0, 1000, LineKind::Fee),
        ("Air Filter", 1.0, 3200, LineKind::Part),
    ];
    lines
        .into_iter()
        .map(|(description, quantity, price_cents, kind)| {
            LineItem::new(String::from(description), quantity, price_cents, kind)
        })
        .collect()
}

/// The date every sample quote is written on. A fixed date (not today's),
/// so the tests print the same text every day.
pub fn quote_time() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 11, 16)
        .and_then(|day| day.and_hms_opt(9, 30, 0))
        .expect("2026-11-16 09:30:00 is a real time")
}

/// The first quote number of the sample year: Q-2026-00000001.
pub fn quote_number() -> QuoteNumber {
    QuoteNumber::first_of_year(2026).expect("2026 is a valid year")
}

/// A fresh ticket machine for the sample year: its first number is Q-2026-00000001.
pub fn quote_numbers() -> InMemoryQuoteNumbers {
    InMemoryQuoteNumbers::for_year(2026)
}

/// A quote made at the branch in `province` for the plain customer and the Corolla,
/// with the standard oil change.
pub fn oil_change_quote(province: Province) -> QuoteResult<Quote> {
    let mut quote = Quote::new(quote_number(), quote_time(), branch_in(province)?)?;
    quote.attach_customer(customer_in(province)?)?;
    quote.attach_equipment(corolla()?)?;
    for item in oil_change_lines()? {
        quote.add_item(item)?;
    }
    Ok(quote)
}
