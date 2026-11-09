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

fn main() {
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
        engineoil_item.description,
        engineoil_item.quantity,
        engineoil_item.unit_price,
        engineoil_item.line_total()
    );
}
