// src/main.rs: a small demo program that uses the Quoto library

use chrono::Local;
use quoto::{
    EquipmentDraft, EquipmentLookup, InMemoryQuoteNumbers, Quote, QuoteNumbers, QuoteResult,
    logging, samples,
};
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
        equipment_source = %quote.equipment().source(),
        "quote printed"
    );
}

/// Asks the lookup about a serial number. This is the one place where a failed
/// lookup stops: it is logged, the person is told, and `None` means "type the
/// details in by hand", the same as when the lookup simply doesn't know the machine.
fn lookup_or_none(lookup: &impl EquipmentLookup, serial: &str) -> Option<EquipmentDraft> {
    match lookup.lookup(serial) {
        Ok(draft) => draft,
        Err(error) => {
            error.log();
            eprintln!("{error}");
            None
        }
    }
}

/// The real work. Any `QuoteError` stops it and is handed back to `main`.
fn run() -> QuoteResult<()> {
    info!("building the demo quotes");
    // The machine hands out numbers one after the other, for the current year:
    // Q-<year>-00000001, Q-<year>-00000002
    let mut numbers = InMemoryQuoteNumbers::new();
    // The clock is read once, here, and handed down: a quote never looks at it.
    let now = Local::now().naive_local();
    // The Corolla's serial number is in the demo catalog, so its make and model
    // come from the lookup. The oilfield truck is not, so it is typed in by hand.
    let lookup = samples::equipment_lookup();
    let corolla = match lookup_or_none(&lookup, samples::COROLLA_SERIAL) {
        Some(draft) => samples::corolla_from(draft)?,
        None => samples::corolla()?,
    };
    show(&samples::demo_quote(
        numbers.next_number()?,
        now,
        samples::prairie_farms()?,
        corolla,
    )?);
    show(&samples::demo_quote(
        numbers.next_number()?,
        now,
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
