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
    fn test_item_line_total() {
        let engine_oil_item: LineItem = LineItem::new(
            String::from("Engine Oil"),
            0.5,
            160.0,
            LineKind::Part
        );
        assert_eq!(engine_oil_item.line_total(), 80.0);
    }
}
