// src/main.rs: a small demo program that uses the Quoto library

use quoto::{Quote, QuoteResult, logging, samples};
use std::process::ExitCode;
use tracing::info;

/// Shows a quote to the person, and logs that it happened.
/// The log holds only the province and the total, never customer details.
fn show(quote: &Quote) {
    println!("{quote}");
    info!(
        province = quote.customer().province().label(),
        total_cents = quote.total(),
        "quote printed"
    );
}

/// The real work. Any `QuoteError` stops it and is handed back to `main`.
fn run() -> QuoteResult<()> {
    info!("building the demo quotes");
    show(&samples::demo_quote(
        samples::prairie_farms()?,
        samples::corolla()?,
    )?);
    show(&samples::demo_quote(
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
