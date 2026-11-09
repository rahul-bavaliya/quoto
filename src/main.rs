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

fn print_quote(province: Province, items: &[LineItem]) {
    let mut parts_total = 0.0;
    let mut labour_total = 0.0;
    let mut fee_total = 0.0;

    println!(
        "{} Quote: {} {}",
        "=".repeat(5),
        province.label(),
        "=".repeat(5)
    );
    for item in items {
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

    let subtotal = parts_total + labour_total + fee_total;
    let gst_amount = calculate_tax(subtotal, province.gst_rate());
    let pst_amount = if let Some(pst_rate) = province.pst_rate() {
        calculate_tax(subtotal, pst_rate)
    } else {
        0.0
    };
    let total = calculate_total(subtotal, gst_amount + pst_amount);
    println!("Subtotal: ${:.2}", subtotal);

    println!("GST {}%: ${:.2}", province.gst_rate(), gst_amount);
    if let Some(pst_rate) = province.pst_rate() {
        println!("PST {}%: ${:.2}", pst_rate, pst_amount);
    }
    println!("Total: ${:.2}", total);
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

    print_quote(Province::Saskatchewan, &items);
    print_quote(Province::Alberta, &items);
}
