// src/main.rs: a small demo program that uses the Quoto library

use quoto::{
    Address, Contact, Customer, Email, Equipment, LineItem, LineKind, Meter, MeterType,
    PhoneNumber, PostalCode, Province, Quote, QuoteError,
};

/// The same job for every demo quote: a fresh set of lines each time,
/// because each quote must own its own lines.
fn sample_items() -> Result<Vec<LineItem>, QuoteError> {
    Ok(vec![
        LineItem::new(String::from("Oil Filter"), 1.0, 2500, LineKind::Part)?,
        LineItem::new(
            String::from("Oil Change Labour"),
            0.5,
            16000,
            LineKind::Labour,
        )?,
        LineItem::new(String::from("Engine Oil"), 6.0, 850, LineKind::Part)?,
        LineItem::new(String::from("Shop Supplies"), 1.0, 1000, LineKind::Fee)?,
        LineItem::new(String::from("Air Filter"), 1.0, 3200, LineKind::Part)?,
        LineItem::new(
            String::from("Loyalty discount"),
            1.0,
            2500,
            LineKind::Credit,
        )?,
        LineItem::new(
            String::from("Goodwill - 3 day wait"),
            1.0,
            5000,
            LineKind::Credit,
        )?,
    ])
}

/// A Saskatchewan customer: GST + PST.
fn prairie_farms() -> Result<Customer, QuoteError> {
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

/// An Alberta customer: GST only, no PST.
fn northern_oilfield() -> Result<Customer, QuoteError> {
    let contact = Contact::new(
        PhoneNumber::parse("780-555-0188")?,
        None,
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

/// A fresh unit each time: a quote owns its equipment, so two quotes
/// can't share one.
fn sample_equipment() -> Equipment {
    Equipment::new(
        String::from("Toyota"),
        String::from("Corolla Cross"),
        Some(2025),
        Some(String::from("SERIAL_NUMBER_TEST_1")),
        Meter::new(Some(28156.98), MeterType::Kilometer),
        None,
        String::from("Car Suv AWD"),
        Some(String::from("RB97")),
    )
}

/// Builds one demo quote: a customer, a unit, and the sample lines.
fn build_quote(customer: Customer) -> Result<Quote, QuoteError> {
    let mut quote = Quote::new(customer, sample_equipment());
    for item in sample_items()? {
        quote.add_item(item)?;
    }
    Ok(quote)
}

fn main() -> Result<(), QuoteError> {
    println!("{}", build_quote(prairie_farms()?)?);
    println!("{}", build_quote(northern_oilfield()?)?);
    Ok(())
}
