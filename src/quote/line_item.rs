// src/quote/line_item.rs

use super::LineKind;
use crate::errors::QuoteError;
use uuid::Uuid;

pub struct LineItem {
    /// The database's id. Empty until this line is loaded from the database:
    /// a new one never makes its own id.
    id: Option<Uuid>,
    /// The id of the quote this line belongs to. Empty until the quote has an id.
    /// Only the quote sets it (see `Quote`), so a line can never point at the wrong quote.
    quote_id: Option<Uuid>,
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
            id: None,
            quote_id: None,
            description,
            quantity,
            unit_price_cents,
            kind,
        })
    }

    /// `None` means "not saved yet". The database gives the id; Quoto never makes one.
    pub fn id(&self) -> Option<Uuid> {
        self.id
    }

    /// Attaches the id the database gave. Used when loading from the database.
    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    /// The quote this line belongs to, once that quote has an id.
    pub fn quote_id(&self) -> Option<Uuid> {
        self.quote_id
    }

    /// Only a quote may say which quote a line belongs to.
    pub(super) fn belong_to(&mut self, quote_id: Uuid) {
        self.quote_id = Some(quote_id);
    }

    pub fn line_total(&self) -> i64 {
        let amount: i64 = (self.quantity * self.unit_price_cents as f64).round() as i64;
        amount * self.sign()
    }
    pub fn signed_unit_price_cents(&self) -> i64 {
        self.unit_price_cents * self.sign()
    }
    /// The price as it was entered (never negative); `signed_unit_price_cents` makes a credit negative.
    pub(crate) fn unit_price_cents(&self) -> i64 {
        self.unit_price_cents
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
