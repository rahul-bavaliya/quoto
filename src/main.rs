mod utils;
use utils::{calculate_tax, calculate_total};

fn main() {
    // Saskatchewan has two taxes: GST goes to Ottawa, PST goes to Regina
    let sask_gst_rate: f64 = 5.0;
    let sask_pst_rate: f64 = 6.0;

    // Alberta has GST only (no PST)
    let alberta_gst_rate: f64 = 5.0;
    let repair_labor: f64 = 80.0;

    println!("----- Quoto System -----");

    // ---------- Saskatchewan ----------
    println!("----- Saskatchewan -----");
    let sk_gst_amount = calculate_tax(repair_labor, sask_gst_rate);
    let sk_pst_amount = calculate_tax(repair_labor, sask_pst_rate);

    println!("Sub Total: ${:.2}", repair_labor);
    println!(
        "GST Rate: {}% | GST $ Amount: ${:.2}",
        sask_gst_rate, sk_gst_amount
    );
    println!(
        "PST Rate: {}% | PST $ Amount: ${:.2}",
        sask_pst_rate, sk_pst_amount
    );
    println!(
        "Total Payable: ${:.2}",
        calculate_total(repair_labor, sk_gst_amount + sk_pst_amount)
    );

    // ---------- Alberta ----------
    println!("----- Alberta -----");
    let ab_gst_amount = calculate_tax(repair_labor, alberta_gst_rate);

    println!("Sub Total: ${:.2}", repair_labor);
    println!(
        "GST Rate: {}% | GST $ Amount: ${:.2}",
        alberta_gst_rate, ab_gst_amount
    );
    println!(
        "Total Payable: ${:.2}",
        calculate_total(repair_labor, ab_gst_amount)
    );
}
