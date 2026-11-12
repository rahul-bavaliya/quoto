// src/line_item/mod.rs

use crate::errors::QuoteError;

pub struct LineItem {
    description: String,
    quantity: f64,
    unit_price_cents: i64,
    kind: LineKind,
}

impl LineItem {
    // +1 for normal lines, -1 for credits (the ONE place the sign is decided)
    fn sign(&self) -> i64 {
        match self.kind {
            LineKind::Credit => -1,
            _ => 1,
        }
    }

    pub fn new(
        description: String,
        quantity: f64,
        unit_price_cents: i64,
        kind: LineKind,
    ) -> Result<Self, QuoteError> {
        if quantity <= 0.0 {
            return Err(QuoteError::BadQuantity { description });
        }
        if unit_price_cents < 0 {
            return Err(QuoteError::NegativePrice { description });
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
        amount * self.sign()
    }
    pub fn signed_unit_price_cents(&self) -> i64 {
        self.unit_price_cents * self.sign()
    }
    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn quantity(&self) -> f64 {
        self.quantity
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

#[cfg(test)]
mod tests;
