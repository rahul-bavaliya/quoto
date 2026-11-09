struct LineItem {
    description: String,
    quantity: f64,
    unit_price: f64,
}

impl LineItem {
    fn line_total(&self) -> f64 {
        self.quantity * self.unit_price
    }
}

mod utils;
use utils::{calculate_tax, calculate_total};

fn main() {
    let sk_gst_rate = 5.00;
    let sk_pst_rate = 6.00;

    let oilfilter_item = LineItem {
        description: String::from("Oil Filter"),
        quantity: 1.0,
        unit_price: 25.0,
    };

    let oilchange_item = LineItem {
        description: String::from("Oil Change Labour"),
        quantity: 0.5,
        unit_price: 160.0,
    };

    let engineoil_item = LineItem {
        description: String::from("Engine Oil"),
        quantity: 6.0,
        unit_price: 8.50,
    };

    println!(
        "{}: {} x ${:.2} = ${:.2}",
        oilchange_item.description,
        oilchange_item.quantity,
        oilchange_item.unit_price,
        oilchange_item.line_total()
    );

    println!(
        "{}: {} x ${:.2} = ${:.2}",
        oilfilter_item.description,
        oilfilter_item.quantity,
        oilfilter_item.unit_price,
        oilfilter_item.line_total()
    );

    println!(
        "{}: {} x ${:.2} = ${:.2}",
        engineoil_item.description,
        engineoil_item.quantity,
        engineoil_item.unit_price,
        engineoil_item.line_total()
    );

    println!("{}", "-".repeat(30));
    println!(
        "Subtotal: ${:.2}",
        oilfilter_item.line_total() + oilchange_item.line_total() + engineoil_item.line_total()
    );
    let gst_amount = calculate_tax(
        oilfilter_item.line_total() + oilchange_item.line_total() + engineoil_item.line_total(),
        sk_gst_rate,
    );
    let pst_amount = calculate_tax(
        oilfilter_item.line_total() + oilchange_item.line_total() + engineoil_item.line_total(),
        sk_pst_rate,
    );
    println!("GST {:.2}%: ${:.2}", sk_gst_rate, gst_amount);
    println!("PST {:.2}%: ${:.2}", sk_pst_rate, pst_amount);
    println!(
        "Total: ${:.2}",
        calculate_total(
            oilfilter_item.line_total() + oilchange_item.line_total() + engineoil_item.line_total(),
            gst_amount + pst_amount
        )
    );
}
