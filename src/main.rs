mod utils;
use utils::{calculate_gst_pst,  calculate_total};

fn main() {
    let gst_rate: f64 = 5.0; // GST rate is 5%
    let pst_rate: f64 = 6.0; // PST rate is 6%

    let repair_labor_billed: f64 = 80.0; // Repair labor billed is $80.00

    println!("----- Quoto System -----"); // Display the title of the program
    let (gst_amount, pst_amount) = calculate_gst_pst(repair_labor_billed, gst_rate, pst_rate);
    println!("GST Rate: {}% | GST $ Amount: ${:.2}", gst_rate, gst_amount); // Display the GST rate and the calculated GST amount
    println!("PST Rate: {}% | PST $ Amount: ${:.2}", pst_rate, pst_amount); // Display the PST rate and the calculated PST amount
    println!("Sub Total: ${:.2}", repair_labor_billed); // Display the subtotal amount
    println!(
        "Total Payable: ${:.2}",
        calculate_total(repair_labor_billed, gst_amount, pst_amount)
    ); // Display the total payable amount
}
