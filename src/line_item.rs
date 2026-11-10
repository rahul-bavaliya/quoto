// src/line_item.rs

pub struct LineItem {
    description: String,
    quantity: f64,
    unit_price: f64,
    kind: LineKind,
}

impl LineItem {
    pub fn new(description: String, quantity: f64, unit_price: f64, kind: LineKind) -> Self {
        LineItem {
            description,
            quantity,
            unit_price,
            kind,
        }
    }

    pub fn line_total(&self) -> f64 {
        self.quantity * self.unit_price
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn quantity(&self) -> f64 {
        self.quantity
    }

    pub fn unit_price(&self) -> f64 {
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
        let labour_item: LineItem = LineItem::new(
            String::from("Oil Change Labour"),
            2.0,
            50.0,
            LineKind::Labour,
        );
        assert_eq!(labour_item.line_total(), 100.0);
    }
}
