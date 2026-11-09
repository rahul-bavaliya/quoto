mod utils;
use utils::{calculate_gst, calculate_pst, calculate_total};

fn main() {

    let gst_rate: f64 = 5.0;
    let pst_rate: f64 = 6.0;

    let repair_labor_billed: f64 = 80.0;
    println!("----- Quoto System -----");
    println!("GST Rate: {}% | GST $ Amount: ${:.2}", gst_rate, calculate_gst(repair_labor_billed, gst_rate));
    println!("PST Rate: {}% | PST $ Amount: ${:.2}", pst_rate, calculate_pst(repair_labor_billed, pst_rate));
    println!("Sub Total: ${:.2}", repair_labor_billed);
    println!("Total Payable: ${:.2}", calculate_total(repair_labor_billed, gst_rate, pst_rate));
}
