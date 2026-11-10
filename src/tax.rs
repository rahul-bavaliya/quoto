// src/tax.rs

pub fn calculate_tax(amount: i64, rate: f64) -> i64 {
    (amount as f64 * rate / 100.0).round() as i64
}

pub fn calculate_total(amount: i64, tax_amount: i64) -> i64 {
    amount + tax_amount   
}
