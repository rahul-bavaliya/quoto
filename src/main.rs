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

    let mut items = vec![
        LineItem {
            description: String::from("Oil Filter"),
            quantity: 1.0,
            unit_price: 25.0,
        },
        LineItem {
            description: String::from("Oil Change Labour"),
            quantity: 0.5,
            unit_price: 160.0,
        },
        LineItem {
            description: String::from("Engine Oil"),
            quantity: 6.0,
            unit_price: 8.50,
        },
    ];

    items.push(LineItem {
        description: String::from("Air Filter"),
        quantity: 1.0,
        unit_price: 32.0,
    });

    let mut subtotal = 0.0;

    for item in &items {
        println!(
            "{}: {} x ${:.2} = ${:.2}",
            item.description,
            item.quantity,
            item.unit_price,
            item.line_total()
        );

        subtotal += item.line_total();
    }

    println!("{}", "-".repeat(30));
    println!("Subtotal: ${:.2}", subtotal);
    let gst_amount = calculate_tax(subtotal, sk_gst_rate);
    let pst_amount = calculate_tax(subtotal, sk_pst_rate);
    println!("GST {:.2}%: ${:.2}", sk_gst_rate, gst_amount);
    println!("PST {:.2}%: ${:.2}", sk_pst_rate, pst_amount);
    println!(
        "Total: ${:.2}",
        calculate_total(subtotal, gst_amount + pst_amount)
    );
}
