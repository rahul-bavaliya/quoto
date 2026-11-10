// src/line_item.rs

pub struct LineItem {
    description: String,
    quantity: f64,
    unit_price_cents: i64,
    kind: LineKind,
}

impl LineItem {
    pub fn new(
        description: String,
        quantity: f64,
        unit_price_cents: i64,
        kind: LineKind,
    ) -> Result<Self, String> {
        if quantity <= 0.0 {
            return Err(format!(
                "{} quantity must be greater than zero.",
                description
            ));
        } else if unit_price_cents < 0 {
            return Err(format!("{} unit price can't be negative.", description));
        }

        Ok(LineItem {
            description,
            quantity,
            unit_price_cents,
            kind,
        })
    }

    pub fn line_total(&self) -> i64 {
        (self.quantity * self.unit_price_cents as f64).round() as i64
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn quantity(&self) -> f64 {
        self.quantity
    }

    pub fn unit_price_cents(&self) -> i64 {
        self.unit_price_cents
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
            LineItem::new(String::from("Oil Change Labour"), 2.0, 50, LineKind::Labour).unwrap();
        assert_eq!(labour_item.line_total(), 100);
    }

    #[test]
    fn three_washers_cost_30_cents() {
        let washers: LineItem =
            LineItem::new(String::from("Washer"), 3.0, 10, LineKind::Part).unwrap();
        assert_eq!(washers.line_total(), 30);
    }

    #[test]
    fn line_item_cent_test() {
        let line_item: LineItem =
            LineItem::new(String::from("Test Item"), 1.5, 833, LineKind::Part).unwrap();
        assert_eq!(line_item.line_total(), 1250);
    }
}
