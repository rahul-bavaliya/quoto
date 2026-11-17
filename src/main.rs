// src/main.rs: a small demo program that uses the Quoto library

use quoto::{Quote, QuoteResult, logging, samples};
use std::process::ExitCode;
use tracing::info;

/// Shows a quote to the person, and logs that it happened.
/// DEMO ONLY: it prints the whole quote (with the customer's details) because
/// the data here is fake. A real server must never print or log a whole quote.
/// The log holds only the number, province and total, never customer details.
fn show(quote: &Quote) {
    println!("{quote}");
    info!(
        quote_number = %quote.number(),
        province = quote.customer().province().label(),
        total_cents = quote.total(),
        "quote printed"
    );
}

/// The real work. Any `QuoteError` stops it and is handed back to `main`.
fn run() -> QuoteResult<()> {
    info!("building the demo quotes");
    // Quote numbers are handed out one after the other: Q-2026-0001, Q-2026-0002
    let first = samples::quote_number();
    let second = first.next()?;
    show(&samples::demo_quote(
        first,
        samples::prairie_farms()?,
        samples::corolla()?,
    )?);
    show(&samples::demo_quote(
        second,
        samples::northern_oilfield()?,
        samples::oilfield_truck()?,
    )?);
    info!("done");
    Ok(())
}

/// The one place where errors stop the program: log it, tell the person,
/// and exit with a code that says what kind of problem it was.
fn main() -> ExitCode {
    logging::init();
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            error.log();
            eprintln!("Quoto could not finish: {error}");
            ExitCode::from(error.exit_code())
        }
    }
}
