// examples/nhtsa_lookup.rs: try the lookups by hand, with the real NHTSA service
//
//     cargo run --example nhtsa_lookup -- 1HGCM82633A004352
//     RUST_LOG=debug cargo run --example nhtsa_lookup -- 1HGCM82633A004352
//
// It asks NHTSA first (road vehicles with a 17-character VIN), then the demo
// catalog (which knows DEMO-8R410-0001, a tractor). It needs the internet, so
// the tests never run it. Use a made-up or public example VIN: don't type a
// customer's machine into a demo.

use quoto::{ChainedLookup, EquipmentLookup, NhtsaLookup, logging, samples};
use std::process::ExitCode;

fn main() -> ExitCode {
    logging::init();
    let Some(serial) = std::env::args().nth(1) else {
        eprintln!("Usage: cargo run --example nhtsa_lookup -- <serial number or VIN>");
        return ExitCode::from(2);
    };

    let lookup = ChainedLookup::new(vec![
        Box::new(NhtsaLookup::new()),
        Box::new(samples::equipment_lookup()),
    ]);
    match lookup.lookup(&serial) {
        Ok(Some(draft)) => println!("Found: {draft:?}"),
        Ok(None) => println!("Nobody knows that serial number: enter the details by hand."),
        Err(error) => {
            error.log();
            println!("{error}");
            return ExitCode::from(error.exit_code());
        }
    }
    ExitCode::SUCCESS
}
