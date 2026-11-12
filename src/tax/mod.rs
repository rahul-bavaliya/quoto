// src/tax/mod.rs

/// Tax on an amount (in cents) at a percentage rate, rounded to the nearest cent.
pub fn calculate_tax(amount: i64, rate: f64) -> i64 {
    (amount as f64 * rate / 100.0).round() as i64
}

#[cfg(test)]
mod tests;
