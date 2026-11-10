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

struct Quote {
    province: Province,
    items: Vec<LineItem>,
}

impl Quote {
    fn new(province: Province) -> Self {
        Quote {
            province,
            items: Vec::new(),
        }
    }

    fn add_item(&mut self, item: LineItem) {
        self.items.push(item);
    }

    fn print(&self) {
        println!("\n{} Quote: {}", "=".repeat(5), self.province.label());

        for item in &self.items {
            println!(
                "[{}] {}: {} x ${:.2} = ${:.2}",
                item.kind.label(),
                item.description,
                item.quantity,
                item.unit_price,
                item.line_total()
            );
        }
        println!("{}", "-".repeat(20));
        println!("Labour Total: ${:.2}", self.labour_total());
        println!("Parts Total: ${:.2}", self.parts_total());
        println!("Fees Total: ${:.2}", self.fees_total());
        println!("{}", "-".repeat(20));
        println!("Subtotal: ${:.2}", self.subtotal());
        println!("{}", "-".repeat(20));
        println!(
            "GST {}%: ${:.2}",
            self.province.gst_rate(),
            self.gst_amount()
        );
        if let Some(pst_rate) = self.province.pst_rate() {
            println!("PST {}%: ${:.2}", pst_rate, self.pst_amount());
        }
        println!("Total: ${:.2}", self.total());
    }

    fn labour_total(&self) -> f64 {
        self.items
            .iter()
            .filter(|item| matches!(item.kind, LineKind::Labour))
            .map(|item| item.line_total())
            .sum()
    }

    fn parts_total(&self) -> f64 {
        self.items
            .iter()
            .filter(|item| matches!(item.kind, LineKind::Part))
            .map(|item| item.line_total())
            .sum()
    }

    fn fees_total(&self) -> f64 {
        self.items
            .iter()
            .filter(|item| matches!(item.kind, LineKind::Fee))
            .map(|item| item.line_total())
            .sum()
    }

    fn subtotal(&self) -> f64 {
        self.labour_total() + self.parts_total() + self.fees_total()
    }

    fn gst_amount(&self) -> f64 {
        calculate_tax(self.subtotal(), self.province.gst_rate())
    }

    fn pst_amount(&self) -> f64 {
        if let Some(pst_rate) = self.province.pst_rate() {
            calculate_tax(self.subtotal(), pst_rate)
        } else {
            0.0
        }
    }

    fn total(&self) -> f64 {
        calculate_total(self.subtotal(), self.gst_amount() + self.pst_amount())
    }
}

mod utils;
use utils::{calculate_tax, calculate_total};

fn sample_items() -> Vec<LineItem> {
    let items = vec![
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
        LineItem {
            description: String::from("Air Filter"),
            quantity: 1.0,
            unit_price: 32.0,
            kind: LineKind::Part,
        },
    ];
    items
}

fn main() {
    let mut sk_quote = Quote::new(Province::Saskatchewan);
    for item in sample_items() {
        sk_quote.add_item(item);
    }
    sk_quote.print();

    let mut ab_quote = Quote::new(Province::Alberta);
    for item in sample_items() {
        ab_quote.add_item(item);
    }
    ab_quote.print();
}
