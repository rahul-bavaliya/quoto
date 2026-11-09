struct LineItem {
    description: String,
    quantity: f64,
    unit_price: f64,
}

fn main() {
    let item = LineItem {
        description: String::from("Oil Filter"),
        quantity: 1.0,
        unit_price: 15.0,
    };

    println!(
        "{}: {} x ${:.2} = ${:.2}",
        item.description,
        item.quantity,
        item.unit_price,
        item.quantity * item.unit_price
    );
}
