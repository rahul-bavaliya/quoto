// src/main.rs: a small demo program that uses the Quoto library

use quoto::errors::QuoteError;
use quoto::line_item::{LineItem, LineKind};
use quoto::province::Province;
use quoto::quote::Quote;

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

fn main() -> Result<(), QuoteError> {
    let mut sk_quote = Quote::new(Province::Saskatchewan);
    for item in sample_items()? {
        sk_quote.add_item(item)?;
    }
    sk_quote.print();

    let mut ab_quote = Quote::new(Province::Alberta);
    for item in sample_items()? {
        ab_quote.add_item(item)?;
    }
    ab_quote.print();
    Ok(())
}
