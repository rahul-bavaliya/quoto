// Turns whole cents into a dollar string: 21978 → "$219.78"
pub fn format_money(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" }; // remember the minus
    let positive = cents.abs(); // -150 → 150
    format!("{}${}.{:02}", sign, positive / 100, positive % 100)
}

#[cfg(test)]
mod tests;
