// Turns whole cents into a dollar string: 21978 → "$219.78"
pub fn format_money(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" }; // remember the minus
    let positive = cents.abs(); // -150 → 150
    format!("{}${}.{:02}", sign, positive / 100, positive % 100)
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
        assert_eq!(format_money(-150), "-$1.50");
    }

    #[test]
    fn tiny_negative_keeps_its_minus() {
        assert_eq!(format_money(-5), "-$0.05");
    }
}
