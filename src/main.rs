struct LineItem {
    description: String,
    quantity: f64,
    unit_price: f64,
    kind: LineKind,
}

impl LineItem {
    fn line_total(&self) -> f64 {
        self.quantity * self.unit_price
    }
}

enum LineKind {
    Labour,
    Part,
    Fee,
}

impl LineKind {
    fn label(&self) -> &str {
        match self {
            LineKind::Labour => "LABOUR",
            LineKind::Part => "PART",
            LineKind::Fee => "FEE",
        }
    }
}

enum Province {
    Saskatchewan,
    Alberta,
}

impl Province {

    fn label(&self) -> &str {
        match self {
            Province::Saskatchewan => "Saskatchewan",
            Province::Alberta => "Alberta",
        }
    }

    fn gst_rate(&self) -> f64 {
        5.0
    }

    fn pst_rate(&self) -> Option<f64> {
        match self {
            Province::Saskatchewan => Some(6.0),
            Province::Alberta => None,
        }
    }
}

mod utils;
use utils::{calculate_tax, calculate_total};

fn main() {
    let mut items = vec![
        LineItem {
            description: String::from("Oil Filter"),
            quantity: 1.0,
            unit_price: 25.0,
            kind: LineKind::Part,
        },
        LineItem {
            description: String::from("Oil Change Labour"),
            quantity: 0.5,
            unit_price: 160.0,
            kind: LineKind::Labour,
        },
        LineItem {
            description: String::from("Engine Oil"),
            quantity: 6.0,
            unit_price: 8.50,
            kind: LineKind::Part,
        },
        LineItem {
            description: String::from("Shop Supplies"),
            quantity: 1.0,
            unit_price: 10.0,
            kind: LineKind::Fee,
        },
    ];

    items.push(LineItem {
        description: String::from("Air Filter"),
        quantity: 1.0,
        unit_price: 32.0,
        kind: LineKind::Part,
    });

    let mut parts_total = 0.0;
    let mut labour_total = 0.0;
    let mut fee_total = 0.0;

    for item in &items {
        println!(
            "[{}] {}: {} x ${:.2} = ${:.2}",
            item.kind.label(),
            item.description,
            item.quantity,
            item.unit_price,
            item.line_total()
        );

        match item.kind {
            LineKind::Labour => labour_total += item.line_total(),
            LineKind::Part => parts_total += item.line_total(),
            LineKind::Fee => fee_total += item.line_total(),
        }
    }

    println!("{}", "-".repeat(30));
    println!("Labour Total: ${:.2}", labour_total);
    println!("Parts Total: ${:.2}", parts_total);
    println!("Fee Total: ${:.2}", fee_total);
    let subtotal = labour_total + parts_total + fee_total;
    println!("Subtotal: ${:.2}", subtotal);

    for province in [Province::Saskatchewan, Province::Alberta] {
        let gst_amount = calculate_tax(subtotal, province.gst_rate());
        let pst_amount = calculate_tax(subtotal, province.pst_rate().unwrap_or(0.0));

        println!(
            "{} | Subtotal: ${} | GST: ${} | PST: ${}, Total: ${}",
            province.label(),
            subtotal,
            gst_amount,
            pst_amount,
            calculate_total(subtotal, gst_amount + pst_amount)
        );
    }
}
