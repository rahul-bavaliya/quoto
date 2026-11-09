
pub fn calculate_gst(repair_labor_billed: f64, gst_rate: f64) -> f64 {
    repair_labor_billed * gst_rate / 100.0

}
pub fn calculate_pst(repair_labor_billed: f64, pst_rate: f64) -> f64 {
    repair_labor_billed * pst_rate / 100.0
}
pub fn calculate_total(repair_labor_billed: f64, gst_rate: f64, pst_rate: f64) -> f64 {
    repair_labor_billed + calculate_gst(repair_labor_billed, gst_rate) + calculate_pst(repair_labor_billed, pst_rate)
}