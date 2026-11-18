// src/main.rs: the program's front door. Quoto is a library for now (see `src/lib.rs`);
// the screens and the web API come in later phases, so this only starts the logger.

use quoto::logging;
use tracing::info;

fn main() {
    logging::init();
    info!("Quoto started. Nothing to run yet: use `cargo test` (the library has no screen).");
}
