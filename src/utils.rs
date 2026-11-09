pub fn calculate_tax(billed_labor: f64, gst_rate: f64, pst_rate: f64) -> (f64, f64) {
    let gst_amount = billed_labor * gst_rate / 100.0;
    let pst_amount = billed_labor * pst_rate / 100.0;
    (gst_amount, pst_amount)
}

pub fn calculate_total(billed_labor: f64, gst_amount: f64, pst_amount: f64) -> f64 {
    billed_labor + gst_amount + pst_amount
}
