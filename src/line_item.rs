// src/line_item.rs

pub struct LineItem {
    description: String,
    quantity: f64,
    unit_price: i64,
    kind: LineKind,
}

impl LineItem {
    pub fn new(description: String, quantity: f64, unit_price: i64, kind: LineKind) -> Self {
        LineItem {
            description,
            quantity,
            unit_price,
            kind,
        }
    }

    pub fn line_total(&self) -> i64 {
        (self.quantity * self.unit_price as f64) as i64
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn quantity(&self) -> f64 {
        self.quantity
    }

    pub fn unit_price(&self) -> i64 {
        self.unit_price
    }

    pub fn kind(&self) -> &LineKind {
        &self.kind
    }
}

pub enum LineKind {
    Labour,
    Part,
    Fee,
}

impl LineKind {
    pub fn label(&self) -> &str {
        match self {
            LineKind::Labour => "LABOUR",
            LineKind::Part => "PART",
            LineKind::Fee => "FEE",
        }
    }
}

#[cfg(test)] // only build this room when running `cargo test`
mod line_item_tests {
    use super::*; // bring in everything from the file above

    #[test]
    fn labour_line_total_is_hours_times_rate() {
        let labour_item: LineItem =
            LineItem::new(String::from("Oil Change Labour"), 2.0, 50, LineKind::Labour);
        assert_eq!(labour_item.line_total(), 100);
    }

    #[test]
    // #[ignore = "f64 can't store $0.10 exactly - fixed in Day 9 (safe money)"]
    fn three_washers_cost_30_cents() {
        let washers: LineItem = LineItem::new(String::from("Washer"), 3.0, 10, LineKind::Part);
        assert_eq!(washers.line_total(), 30);
    }
}
