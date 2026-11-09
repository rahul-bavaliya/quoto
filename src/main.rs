mod utils;
use utils::{calculate_tax,  calculate_total};

fn main() {
    let sask_gst_rate: f64 = 5.0; // GST rate is 5%
    let sask_pst_rate: f64 = 0.0; // PST rate is 0%

    let alberta_gst_rate: f64 = 5.0; // GST rate is 5%
    let alberta_pst_rate: f64 = 0.0; // PST rate is 0%

    let repair_labor : f64 = 80.0; // Repair labor billed is $80.00

    println!("----- Quoto System -----"); // Display the title of the program
    println!("----- Saskatchewan -----"); // Display the title for Saskatchewan
    let (gst_amount, pst_amount) = calculate_tax(repair_labor, sask_gst_rate, sask_pst_rate);
    println!("GST Rate: {}% | GST $ Amount: ${:.2}", sask_gst_rate, gst_amount); // Display the GST rate and the calculated GST amount
    println!("PST Rate: {}% | PST $ Amount: ${:.2}", sask_pst_rate, pst_amount); // Display the PST rate and the calculated PST amount
    println!("Sub Total: ${:.2}", repair_labor); // Display the subtotal amount
    println!(
        "Total Payable: ${:.2}",
        calculate_total(repair_labor, gst_amount, pst_amount)
    ); // Display the total payable amount


    println!("----- Alberta -----"); // Display the title for Alberta
    let (gst_amount, pst_amount) = calculate_tax(repair_labor, alberta_gst_rate, alberta_pst_rate);
    println!("GST Rate: {}% | GST $ Amount: ${:.2}", alberta_gst_rate, gst_amount); // Display the GST rate and the calculated GST amount for Alberta
    println!("PST Rate: {}% | PST $ Amount: ${:.2}", alberta_pst_rate, pst_amount); // Display the PST rate and the calculated PST amount for Alberta
    println!("Sub Total: ${:.2}", repair_labor); // Display the subtotal amount
    println!(
        "Total Payable: ${:.2}",
        calculate_total(repair_labor, gst_amount, pst_amount)
    ); // Display the total payable amount
}
