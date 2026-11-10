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
        let amount: i64 = (self.quantity * self.unit_price_cents as f64).round() as i64;
        match self.kind {
            LineKind::Credit => -amount,
            _ => amount,
        }
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
    Credit,
}

impl LineKind {
    pub fn label(&self) -> &str {
        match self {
            LineKind::Labour => "LABOUR",
            LineKind::Part => "PART",
            LineKind::Fee => "FEE",
            LineKind::Credit => "CREDIT",
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

    #[test]
    fn negative_quantity_is_refused() {
        let result = LineItem::new(String::from("Oil Filter"), -2.0, 2500, LineKind::Part);
        assert!(result.is_err());
    }

    #[test]
    fn negative_price_is_refused() {
        let result = LineItem::new(String::from("Oil Filter"), 2.0, -2500, LineKind::Part);
        assert!(result.is_err());
    }

    #[test]
    fn zero_price_is_allowed() {
        let result = LineItem::new(String::from("Free Warranty Part"), 2.0, 0, LineKind::Part);
        assert!(result.is_ok());
    }

    #[test]
    fn zero_quantity_is_refused() {
        let result = LineItem::new(String::from("Oil Filter"), 0.0, 2500, LineKind::Part);
        assert!(result.is_err());
    }

    #[test]
    fn credit_line_total_is_negative() {
        let credit = LineItem::new(String::from("Goodwill"), 1.0, 5000, LineKind::Credit).unwrap();
        assert_eq!(credit.line_total(), -5000);
    }
}
