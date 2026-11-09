pub fn calculate_tax(amount: f64, rate: f64) -> f64 {
    let tax_amount = amount * rate / 100.0;
    tax_amount
}

pub fn calculate_total(amount: f64, tax_amount: f64) -> f64 {
    amount + tax_amount
}
