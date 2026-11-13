// src/quote.rs

use crate::customer::Customer;
use crate::equipment::Equipment;
use crate::errors::QuoteError;
use crate::line_item::{LineItem, LineKind};
use crate::money::format_money;
use crate::province::Province;
use crate::tax::calculate_tax;
use std::fmt;

pub struct Quote {
    customer: Customer,
    equipment: Equipment,
    items: Vec<LineItem>,
}

impl Quote {
    pub fn new(customer: Customer, equipment: Equipment) -> Self {
        Quote {
            customer,
            equipment,
            items: Vec::new(),
        }
    }

    /// Adds a line to the quote. A credit is refused if it would push
    /// the subtotal below zero (the dealer would owe the customer).
    pub fn add_item(&mut self, item: LineItem) -> Result<(), QuoteError> {
        let subtotal_after = self.subtotal() + item.line_total();
        if matches!(item.kind(), LineKind::Credit) && subtotal_after < 0 {
            return Err(QuoteError::CreditTooLarge {
                credit_cents: -item.line_total(),
                subtotal_cents: self.subtotal(),
            });
        }

        self.items.push(item);
        Ok(())
    }

    pub fn customer(&self) -> &Customer {
        &self.customer
    }

    /// The customer's province decides the taxes (one source of truth).
    fn province(&self) -> &Province {
        self.customer.province()
    }

    /// Adds up every line of one kind (e.g. all the parts).
    pub fn total_for(&self, kind: &LineKind) -> i64 {
        self.items
            .iter()
            .filter(|item| item.kind() == kind)
            .map(|item| item.line_total())
            .sum()
    }

    pub fn labour_total(&self) -> i64 {
        self.total_for(&LineKind::Labour)
    }

    pub fn parts_total(&self) -> i64 {
        self.total_for(&LineKind::Part)
    }

    pub fn fees_total(&self) -> i64 {
        self.total_for(&LineKind::Fee)
    }

    /// Credits are negative, so this is zero or below.
    pub fn credit_total(&self) -> i64 {
        self.total_for(&LineKind::Credit)
    }

    pub fn subtotal(&self) -> i64 {
        self.labour_total() + self.parts_total() + self.fees_total() + self.credit_total()
    }

    pub fn gst_amount(&self) -> i64 {
        calculate_tax(self.subtotal(), self.province().gst_rate())
    }

    pub fn pst_amount(&self) -> i64 {
        if let Some(pst_rate) = self.province().pst_rate() {
            calculate_tax(self.subtotal(), pst_rate)
        } else {
            0
        }
    }

    pub fn total(&self) -> i64 {
        self.subtotal() + self.gst_amount() + self.pst_amount()
    }
}

/// Shows a quote the way a customer reads it. Because this writes into
/// `f` instead of printing, the same text can go to the terminal, a web
/// page or a PDF later.
impl fmt::Display for Quote {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f)?;
        writeln!(
            f,
            "{} Quote: {} {}",
            "=".repeat(5),
            self.province().label(),
            "=".repeat(5)
        )?;
        writeln!(f, "Customer: {}", self.customer)?;
        writeln!(f, "{} ", "-".repeat(20),)?;
        writeln!(f, "Equipment: {}", self.equipment)?;

        for item in &self.items {
            writeln!(
                f,
                "[{}] {}: {} x {} = {}",
                item.kind().label(),
                item.description(),
                item.quantity(),
                format_money(item.signed_unit_price_cents()),
                format_money(item.line_total())
            )?;
        }
        writeln!(f, "{}", "-".repeat(20))?;
        writeln!(f, "Labour Total: {}", format_money(self.labour_total()))?;
        writeln!(f, "Parts Total: {}", format_money(self.parts_total()))?;
        writeln!(f, "Fees Total: {}", format_money(self.fees_total()))?;
        writeln!(f, "Credit Total: {}", format_money(self.credit_total()))?;
        writeln!(f, "{}", "-".repeat(20))?;
        writeln!(f, "Subtotal: {}", format_money(self.subtotal()))?;
        writeln!(f, "{}", "-".repeat(20))?;
        writeln!(
            f,
            "GST {}%: {}",
            self.province().gst_rate(),
            format_money(self.gst_amount())
        )?;
        if let Some(pst_rate) = self.province().pst_rate() {
            writeln!(f, "PST {}%: {}", pst_rate, format_money(self.pst_amount()))?;
        }
        write!(f, "Total: {}", format_money(self.total()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MeterType;
    use crate::address::{Address, PostalCode};
    use crate::contact::{Contact, PhoneNumber};

    /// A customer who lives in the given province.
    fn sample_customer(province: Province) -> Customer {
        let (city, postal_code) = match province {
            Province::Saskatchewan => ("Regina", "S4P 3Y2"),
            Province::Alberta => ("Calgary", "T2P 1J9"),
        };
        let contact = Contact::new(PhoneNumber::parse("306-555-0142").unwrap(), None, None);
        let address = Address::new(
            String::from("123 Main St"),
            None,
            String::from(city),
            PostalCode::parse(postal_code).unwrap(),
            province,
        )
        .unwrap();
        Customer::new(String::from("Jane Smith"), None, None, contact, address).unwrap()
    }

    /// A customer who lives in the given province.
    fn sample_equipment() -> Equipment {
        Equipment::new(
            String::from("Toyota"),
            String::from("Corolla Cross"),
            2025,
            Some(String::from("SERIAL_NUMBER_TEST_1")),
            28156.0,
            None,
            MeterType::Kilometer,
            String::from("Car Suv AWD"),
            String::from("RB97"),
        )
    }

    // The 5 standard lines. No credits here: credit robots add their own.
    fn sample_quote(province: Province) -> Quote {
        let mut quote = Quote::new(sample_customer(province), sample_equipment());
        let lines = [
            ("Oil Filter", 1.0, 2500, LineKind::Part),
            ("Oil Change Labour", 0.5, 16000, LineKind::Labour),
            ("Engine Oil", 6.0, 850, LineKind::Part),
            ("Shop Supplies", 1.0, 1000, LineKind::Fee),
            ("Air Filter", 1.0, 3200, LineKind::Part),
        ];
        for (description, quantity, price_cents, kind) in lines {
            let item =
                LineItem::new(String::from(description), quantity, price_cents, kind).unwrap();
            quote.add_item(item).unwrap();
        }
        quote
    }

    // A credit of any size, for the credit robots
    fn credit(cents: i64) -> LineItem {
        LineItem::new(String::from("Goodwill"), 1.0, cents, LineKind::Credit).unwrap()
    }

    #[test]
    fn sk_quote_subtotal() {
        let quote = sample_quote(Province::Saskatchewan);
        assert_eq!(quote.subtotal(), 19800);
    }

    #[test]
    fn ab_quote_has_no_pst() {
        let quote = sample_quote(Province::Alberta);
        assert_eq!(quote.pst_amount(), 0);
    }

    #[test]
    fn sk_quote_gst_cents() {
        let quote = sample_quote(Province::Saskatchewan);
        assert_eq!(quote.gst_amount(), 990);
    }

    #[test]
    fn sk_quote_pst() {
        let quote = sample_quote(Province::Saskatchewan);
        assert_eq!(quote.pst_amount(), 1188);
    }

    #[test]
    fn new_quote_is_empty() {
        let quote = Quote::new(sample_customer(Province::Alberta), sample_equipment());
        assert!(quote.items.is_empty());
    }

    #[test]
    fn quote_takes_its_province_from_the_customer() {
        let quote = Quote::new(sample_customer(Province::Alberta), sample_equipment());
        assert_eq!(quote.province(), &Province::Alberta);
    }

    #[test]
    fn plain_quote_has_no_credits() {
        let quote = sample_quote(Province::Saskatchewan);
        assert_eq!(quote.credit_total(), 0);
    }

    #[test]
    fn credit_comes_off_before_tax() {
        let mut quote = sample_quote(Province::Saskatchewan);
        quote.add_item(credit(5000)).unwrap();
        assert_eq!(quote.credit_total(), -5000);
        assert_eq!(quote.subtotal(), 14800); // 198.00 - 50.00
        assert_eq!(quote.gst_amount(), 740); // 5% of 148.00, not of 198.00
    }

    #[test]
    fn total_for_adds_only_that_kind() {
        let quote = sample_quote(Province::Saskatchewan);
        assert_eq!(quote.total_for(&LineKind::Part), 10800); // 25.00 + 51.00 + 32.00
        assert_eq!(quote.total_for(&LineKind::Labour), 8000);
        assert_eq!(quote.total_for(&LineKind::Fee), 1000);
        assert_eq!(quote.total_for(&LineKind::Credit), 0);
    }
}
