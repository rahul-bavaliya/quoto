// tests/tax_rates.rs: the rules for tax rates (public API only; no database needed)

use quoto::*;

fn rate(thousandths: i32) -> TaxRate {
    TaxRate::from_thousandths(thousandths).unwrap()
}

// ===== one rate =====

#[test]
fn a_rate_is_kept_as_thousandths_of_a_percent() {
    assert_eq!(rate(5000).thousandths(), 5000); // 5%
    assert_eq!(rate(9975).thousandths(), 9975); // 9.975%
}

#[test]
fn the_smallest_and_the_biggest_rate_are_allowed() {
    assert!(TaxRate::from_thousandths(1).is_ok());
    assert!(TaxRate::from_thousandths(100_000).is_ok());
}

#[test]
fn zero_negative_and_over_a_hundred_percent_are_refused() {
    for value in [0, -1, 100_001] {
        assert_eq!(
            TaxRate::from_thousandths(value),
            Err(QuoteError::InvalidTaxRate { value }),
            "{value}"
        );
    }
}

// ===== a province's taxes =====

#[test]
fn a_gst_alone_a_gst_with_a_pst_and_an_hst_are_allowed() {
    assert_eq!(
        TaxRates::from_columns(Some(5000), None, None),
        Ok(TaxRates::GstPst {
            gst: rate(5000),
            pst: None
        })
    );
    assert_eq!(
        TaxRates::from_columns(Some(5000), Some(6000), None),
        Ok(TaxRates::GstPst {
            gst: rate(5000),
            pst: Some(rate(6000))
        })
    );
    assert_eq!(
        TaxRates::from_columns(None, None, Some(13_000)),
        Ok(TaxRates::Hst(rate(13_000)))
    );
}

#[test]
fn a_gst_and_an_hst_together_are_refused() {
    assert_eq!(
        TaxRates::from_columns(Some(5000), None, Some(13_000)),
        Err(QuoteError::InvalidTaxSystem)
    );
}

#[test]
fn an_hst_with_a_pst_is_refused() {
    assert_eq!(
        TaxRates::from_columns(None, Some(6000), Some(13_000)),
        Err(QuoteError::InvalidTaxSystem)
    );
}

#[test]
fn a_province_with_no_tax_at_all_is_refused() {
    assert_eq!(
        TaxRates::from_columns(None, None, None),
        Err(QuoteError::InvalidTaxSystem)
    );
}

#[test]
fn a_pst_without_a_gst_is_refused() {
    assert_eq!(
        TaxRates::from_columns(None, Some(6000), None),
        Err(QuoteError::InvalidTaxSystem)
    );
}

#[test]
fn a_bad_rate_inside_a_row_is_refused_too() {
    assert_eq!(
        TaxRates::from_columns(Some(0), None, None),
        Err(QuoteError::InvalidTaxRate { value: 0 })
    );
}

#[test]
fn the_columns_can_be_read_back_exactly() {
    for columns in [
        (Some(5000), None, None),
        (Some(5000), Some(9975), None),
        (None, None, Some(15_000)),
    ] {
        let rates = TaxRates::from_columns(columns.0, columns.1, columns.2).unwrap();
        assert_eq!(rates.to_columns(), columns);
    }
}
