// src/main.rs

mod line_item;
mod money;
mod province;
mod quote;
mod tax;

use line_item::{LineItem, LineKind};
use province::Province;
use quote::Quote;

fn sample_items() -> Vec<LineItem> {
    vec![
        LineItem::new(String::from("Oil Filter"), 2.0, 2500, LineKind::Part).unwrap(),
        LineItem::new(
            String::from("Oil Change Labour"),
            0.5,
            16000,
            LineKind::Labour,
        )
        .unwrap(),
        LineItem::new(String::from("Engine Oil"), 6.0, 850, LineKind::Part).unwrap(),
        LineItem::new(String::from("Shop Supplies"), 1.0, 1000, LineKind::Fee).unwrap(),
        LineItem::new(String::from("Air Filter"), 1.0, 3200, LineKind::Part).unwrap(),
    ]
}

fn main() {
    let mut sk_quote: Quote = Quote::new(Province::Saskatchewan);
    for item in sample_items() {
        sk_quote.add_item(item);
    }
    sk_quote.print();

    let mut ab_quote: Quote = Quote::new(Province::Alberta);
    for item in sample_items() {
        ab_quote.add_item(item);
    }
    ab_quote.print();
}
