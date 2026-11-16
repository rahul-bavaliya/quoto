// tests/logging.rs: tests for the logging feature (public API only)

use quoto::logging::init;

// ===== logging =====

#[test]
fn starting_the_logger_twice_is_harmless() {
    init();
    init();
}
