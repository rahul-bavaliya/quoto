// src/main.rs

mod line_item;
mod province;
mod quote;
mod tax;

use line_item::{LineItem, LineKind};
use province::Province;
use quote::Quote;

fn sample_items() -> Vec<LineItem> {
    vec![
        LineItem::new(String::from("Oil Filter"), 1.0, 25.0, LineKind::Part),
        LineItem::new(
            String::from("Oil Change Labour"),
            0.5,
            160.0,
            LineKind::Labour,
        ),
        LineItem::new(String::from("Engine Oil"), 6.0, 8.50, LineKind::Part),
        LineItem::new(String::from("Shop Supplies"), 1.0, 10.0, LineKind::Fee),
        LineItem::new(String::from("Air Filter"), 1.0, 32.0, LineKind::Part),
    ]
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
