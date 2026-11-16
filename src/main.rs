// src/main.rs: a small demo program that uses the Quoto library

use quoto::{
    Address, Contact, Customer, Email, Equipment, LineItem, LineKind, MeterType, PhoneNumber,
    PostalCode, Province, Quote, QuoteError, equipment::Meter,
};

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

fn main() -> Result<(), QuoteError> {
    let equipment = Equipment::new(
        String::from("Toyota"),
        String::from("Corolla Cross"),
        Some(2025),
        Some(String::from("SERIAL_NUMBER_TEST_1")),
        Meter::new(Some(28156.98), MeterType::Kilometer),
        None,
        String::from("Car Suv AWD"),
        Some(String::from("RB97")),
    );
    let mut sk_quote = Quote::new(prairie_farms()?, equipment);
    for item in sample_items()? {
        sk_quote.add_item(item)?;
    }
    println!("{}", sk_quote);

    Ok(())
}
