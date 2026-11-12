// src/tax/tests.rs: unit tests for tax (they can see private fields)

use super::*;

#[test]
fn calculate_tax_amounts() {
    assert_eq!(calculate_tax(100, 10.0), 10);
    assert_eq!(calculate_tax(100, 5.5), 6);
}
