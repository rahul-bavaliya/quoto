// src/logging.rs: one logging setup for the whole program

use std::io::IsTerminal;
use tracing_subscriber::EnvFilter;

/// Starts the logger. Call it once, first thing in `main`.
///
/// - Logs go to **stderr**, so the quote text on stdout stays clean.
/// - The level comes from the `RUST_LOG` environment variable
///   (`RUST_LOG=debug cargo run`). With nothing set, it shows `info` and above.
/// - Calling it twice is harmless (the second call does nothing).
pub fn init() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        // Colours only when a person is watching, not in files or CI logs
        .with_ansi(std::io::stderr().is_terminal())
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starting_the_logger_twice_is_harmless() {
        init();
        init();
    }
}
