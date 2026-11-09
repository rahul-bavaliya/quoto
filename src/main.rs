fn main() {

    let gst_rate = 5;
    let pst_rate = 6;

    let repair_labor_billed = 80;
    println!("----- Quoto System -----");
    println!("GST Rate: {}%", gst_rate);
    println!("PST Rate: {}%", pst_rate);
    println!("Repair Labor Billed: ${}", repair_labor_billed);
    println!("Total Payable: ${:.2}", (repair_labor_billed + (repair_labor_billed * gst_rate / 100) + (repair_labor_billed * pst_rate / 100)) as f64);
}
