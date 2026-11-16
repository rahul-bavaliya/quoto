// tests/tax.rs: tests for the tax feature (public API only)

use quoto::tax::calculate_tax;

// ===== tax =====

#[test]
fn calculate_tax_amounts() {
    assert_eq!(calculate_tax(100, 10.0), 10);
    assert_eq!(calculate_tax(100, 5.5), 6);
}
