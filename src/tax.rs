// src/tax.rs

pub fn calculate_tax(amount: f64, rate: f64) -> f64 {
    amount * rate / 100.0
}

pub fn calculate_total(amount: f64, tax_amount: f64) -> f64 {
    amount + tax_amount
}
