// examples/quote_pdf.rs: write the sample quote as a PDF to look at (made-up people only)
//
//     cargo run --example quote_pdf            # writes quote.pdf here
//     cargo run --example quote_pdf out.pdf

use chrono::NaiveDate;
use quoto::{Province, samples};

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "quote.pdf".to_string());
    let quote = samples::oil_change_quote(Province::Saskatchewan).expect("the sample quote");
    let today = NaiveDate::from_ymd_opt(2026, 11, 18).expect("a real day");
    std::fs::write(&path, quote.to_pdf(today)).expect("write the file");
    println!("wrote {path}");
}
