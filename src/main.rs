// src/main.rs

mod line_item;
mod province;
mod quote;
mod tax;

use line_item::{LineItem, LineKind};
use province::Province;
use quote::Quote;

fn sample_items() -> Vec<LineItem> {
    let items = vec![
        LineItem {
            description: String::from("Oil Filter"),
            quantity: 1.0,
            unit_price: 25.0,
            kind: LineKind::Part,
        },
        LineItem {
            description: String::from("Oil Change Labour"),
            quantity: 0.5,
            unit_price: 160.0,
            kind: LineKind::Labour,
        },
        LineItem {
            description: String::from("Engine Oil"),
            quantity: 6.0,
            unit_price: 8.50,
            kind: LineKind::Part,
        },
        LineItem {
            description: String::from("Shop Supplies"),
            quantity: 1.0,
            unit_price: 10.0,
            kind: LineKind::Fee,
        },
        LineItem {
            description: String::from("Air Filter"),
            quantity: 1.0,
            unit_price: 32.0,
            kind: LineKind::Part,
        },
    ];
    items
}

fn main() {
    let mut sk_quote = Quote::new(Province::Saskatchewan);
    for item in sample_items() {
        sk_quote.add_item(item);
    }
    sk_quote.print();

    let mut ab_quote = Quote::new(Province::Alberta);
    for item in sample_items() {
        ab_quote.add_item(item);
    }
    ab_quote.print();
}
