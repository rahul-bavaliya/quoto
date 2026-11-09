mod utils;
use utils::{calculate_tax, calculate_total};

fn main() {
    let sask_tax_rate: f64 = 11.0; // Total tax rate for Saskatchewan (5% GST + 6% PST)

    let alberta_tax_rate: f64 = 5.0; // Total tax rate for Alberta (5% GST + 0% PST)

    let repair_labor: f64 = 80.0; // Repair labor billed is $80.00

    println!("----- Quoto System -----"); // Display the title of the program
    println!("----- Saskatchewan -----"); // Display the title for Saskatchewan
    let sk_tax_amount = calculate_tax(repair_labor, sask_tax_rate); // Calculate the total tax amount for Saskatchewan
    println!(
        "Tax Rate: {}% | Tax $ Amount: ${:.2}",
        sask_tax_rate, sk_tax_amount
    ); // Display the tax rate and the calculated tax amount
    println!("Sub Total: ${:.2}", repair_labor); // Display the subtotal amount
    println!(
        "Total Payable: ${:.2}",
        calculate_total(repair_labor, sk_tax_amount)
    ); // Display the total payable amount

    println!("----- Alberta -----"); // Display the title for Alberta
    let ab_tax_amount = calculate_tax(repair_labor, alberta_tax_rate);
    println!(
        "Tax Rate: {}% | Tax $ Amount: ${:.2}",
        alberta_tax_rate, ab_tax_amount
    ); // Display the tax rate and the calculated tax amount for Alberta
    println!("Sub Total: ${:.2}", repair_labor); // Display the subtotal amount
    println!(
        "Total Payable: ${:.2}",
        calculate_total(repair_labor, ab_tax_amount)
    ); // Display the total payable amount
}
