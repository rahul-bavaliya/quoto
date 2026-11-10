// Turns whole cents into a dollar string: 21978 → "$219.78"
pub fn format_money(cents: i64) -> String {
    let dollars: i64 = cents / 100; // full dollars: 21978 / 100 = 219
    let pennies: i64 = cents % 100; // leftover cents: 21978 % 100 = 78
    if pennies < 0 {
        // Handle negative cents correctly
        let pennies: i64 = -pennies;
        return format!("${}.{:02}", dollars, pennies);
    }
    format!("${}.{:02}", dollars, pennies)
}

#[cfg(test)]
mod money_tests {
    use super::*;

    #[test]
    fn formats_dollars_and_cents() {
        assert_eq!(format_money(21978), "$219.78");
    }

    #[test]
    fn pads_single_digit_cents() {
        assert_eq!(format_money(105), "$1.05");
    }

    #[test]
    fn formats_zero() {
        assert_eq!(format_money(0), "$0.00");
    }

    #[test]
    fn negative_amounts() {
        assert_eq!(format_money(-150), "$-1.50");
    }
}
