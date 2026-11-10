// src/tax.rs

pub fn calculate_tax(amount: i64, rate: f64) -> i64 {
    (amount as f64 * rate / 100.0).round() as i64
}

pub fn calculate_total(amount: i64, tax_amount: i64) -> i64 {
    amount + tax_amount
}

#[cfg(test)] // only build this room when running `cargo test`
mod tax_tests {
    use super::*; // bring in everything from the file above

    #[test]
    fn calculate_tax_amounts() {
        assert_eq!(calculate_tax(100, 10.0), 10);
        assert_eq!(calculate_tax(100, 5.5), 6);
    }
}
