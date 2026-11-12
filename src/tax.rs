// src/tax.rs

/// Tax on an amount (in cents) at a percentage rate, rounded to the nearest cent.
pub fn calculate_tax(amount: i64, rate: f64) -> i64 {
    (amount as f64 * rate / 100.0).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_tax_amounts() {
        assert_eq!(calculate_tax(100, 10.0), 10);
        assert_eq!(calculate_tax(100, 5.5), 6);
    }
}
