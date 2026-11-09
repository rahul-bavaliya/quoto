pub fn calculate_tax(billed_labor: f64, tax: f64) -> f64 {
    let tax_amount = billed_labor * tax / 100.0;
    tax_amount
}

pub fn calculate_total(billed_labor: f64, tax_amount: f64) -> f64 {
    billed_labor + tax_amount
}
