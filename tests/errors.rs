// tests/errors.rs: tests for the errors feature (public API only)

use quoto::*;

// ===== errors =====

#[test]
fn credit_too_large_message_shows_dollars() {
    let error = QuoteError::CreditTooLarge {
        credit_cents: 30000,
        subtotal_cents: 19800,
    };
    assert_eq!(
        error.to_string(),
        "Credit of $300.00 is larger than the subtotal of $198.00."
    );
}

#[test]
fn every_message_reads_like_a_sentence() {
    assert_eq!(
        QuoteError::BadQuantity {
            description: String::from("Oil Filter")
        }
        .to_string(),
        "A line's quantity must be greater than zero."
    );
    assert_eq!(
        QuoteError::NegativePrice {
            description: String::from("Oil Filter")
        }
        .to_string(),
        "A line's unit price can't be negative."
    );
    assert_eq!(
        QuoteError::MissingField { field: "city" }.to_string(),
        "city is required."
    );
    assert_eq!(
        QuoteError::InvalidPhone {
            input: String::from("555")
        }
        .to_string(),
        "That is not a valid 10-digit phone number."
    );
    assert_eq!(
        QuoteError::InvalidEmail {
            input: String::from("bob")
        }
        .to_string(),
        "That is not a valid email address."
    );
    assert_eq!(
        QuoteError::InvalidPostalCode {
            input: String::from("12345")
        }
        .to_string(),
        "That is not a valid postal code (like S4P 3Y2)."
    );
    assert_eq!(
        QuoteError::InvalidQuoteNumber {
            input: String::from("Q-1")
        }
        .to_string(),
        "That is not a valid quote number (like Q-2026-00000001)."
    );
    assert_eq!(
        QuoteError::QuoteNumbersExhausted { year: 2026 }.to_string(),
        "All the quote numbers for 2026 are used up."
    );
    assert_eq!(
        QuoteError::MissingIdentification.to_string(),
        "A serial number or a plate is required to identify the unit."
    );
    assert_eq!(
        QuoteError::InvalidMeterReading { reading: -5.0 }.to_string(),
        "The meter reading must be a number that is zero or more."
    );
}

#[test]
fn a_refused_credit_is_a_business_rule() {
    let error = QuoteError::CreditTooLarge {
        credit_cents: 100,
        subtotal_cents: 50,
    };
    assert_eq!(error.kind(), ErrorKind::BusinessRule);
    assert_eq!(error.exit_code(), 3);
}

#[test]
fn running_out_of_quote_numbers_is_a_business_rule() {
    let error = QuoteError::QuoteNumbersExhausted { year: 2026 };
    assert_eq!(error.kind(), ErrorKind::BusinessRule);
}

#[test]
fn a_typo_is_invalid_input() {
    let error = QuoteError::InvalidEmail {
        input: String::from("bob"),
    };
    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert_eq!(error.exit_code(), 2);
}

#[test]
fn the_quote_party_errors_are_business_rules() {
    for error in [
        QuoteError::AlreadyAttached { what: "customer" },
        QuoteError::EquipmentNeedsCustomer,
        QuoteError::EquipmentNotOwnedByCustomer,
        QuoteError::QuoteIncomplete {
            missing: "customer",
        },
        QuoteError::BranchInactive,
    ] {
        assert_eq!(error.kind(), ErrorKind::BusinessRule);
    }
}

#[test]
fn a_failed_database_set_up_is_a_service_problem() {
    assert_eq!(
        QuoteError::DatabaseSetupFailed.kind(),
        ErrorKind::ServiceUnavailable
    );
    // The message is safe to show: it names no table, address or password.
    assert!(
        !QuoteError::DatabaseSetupFailed
            .to_string()
            .contains("postgres")
    );
}

#[test]
fn the_saved_quote_errors_have_the_right_kind() {
    for error in [
        QuoteError::QuoteAlreadySaved,
        QuoteError::QuoteNotSaved,
        QuoteError::QuoteChangedMeanwhile,
    ] {
        assert_eq!(error.kind(), ErrorKind::BusinessRule);
    }
    assert_eq!(QuoteError::UnknownQuote.kind(), ErrorKind::InvalidInput);
    assert_eq!(
        QuoteError::StoredQuoteDamaged.kind(),
        ErrorKind::ServiceUnavailable
    );
}

#[test]
fn the_repair_order_errors_have_the_right_kind() {
    for error in [
        QuoteError::RepairOrderNotSaved,
        QuoteError::QuoteNotAccepted,
        QuoteError::RepairOrderExists,
        QuoteError::RepairOrderNotOpen,
        QuoteError::RepairOrderNotInProgress,
        QuoteError::RepairOrderClosed,
        QuoteError::RepairOrderChangedMeanwhile,
    ] {
        assert_eq!(error.kind(), ErrorKind::BusinessRule, "{error:?}");
    }
    for error in [
        QuoteError::UnknownRepairOrder,
        QuoteError::MeterReadingRequired,
        QuoteError::DueDateBeforeOpened,
        QuoteError::CancellationReasonTooLong { max: 500 },
        QuoteError::InvalidRepairOrderNumber { input: "x".into() },
    ] {
        assert_eq!(error.kind(), ErrorKind::InvalidInput, "{error:?}");
    }
    assert_eq!(
        QuoteError::StoredRepairOrderDamaged.kind(),
        ErrorKind::ServiceUnavailable
    );
}

#[test]
fn the_equipment_errors_have_the_right_kind() {
    for error in [
        QuoteError::EquipmentAlreadySaved,
        QuoteError::EquipmentNotSaved,
        QuoteError::EquipmentInactive,
        QuoteError::DuplicateSerialNumber,
    ] {
        assert_eq!(error.kind(), ErrorKind::BusinessRule);
    }
    for error in [
        QuoteError::UnknownEquipment,
        QuoteError::InvalidEquipmentSource,
        QuoteError::InvalidMeterType { value: "x".into() },
        QuoteError::InvalidEquipmentType { value: "x".into() },
    ] {
        assert_eq!(error.kind(), ErrorKind::InvalidInput);
    }
}

#[test]
fn the_customer_save_errors_are_business_rules_and_a_missing_customer_is_wrong_input() {
    for error in [
        QuoteError::CustomerAlreadySaved,
        QuoteError::CustomerNotSaved,
        QuoteError::CustomerInactive,
    ] {
        assert_eq!(error.kind(), ErrorKind::BusinessRule);
    }
    assert_eq!(QuoteError::UnknownCustomer.kind(), ErrorKind::InvalidInput);
}

#[test]
fn a_server_that_cannot_start_is_an_unavailable_service() {
    assert_eq!(
        QuoteError::ServerStartFailed.kind(),
        ErrorKind::ServiceUnavailable
    );
}

#[test]
fn the_branch_save_errors_are_business_rules_and_a_missing_branch_is_wrong_input() {
    for error in [
        QuoteError::DuplicateBranchNumber { number: "1".into() },
        QuoteError::BranchAlreadySaved,
        QuoteError::BranchNotSaved,
    ] {
        assert_eq!(error.kind(), ErrorKind::BusinessRule);
    }
    assert_eq!(QuoteError::UnknownBranch.kind(), ErrorKind::InvalidInput);
    assert_eq!(
        QuoteError::InvalidStatus { value: "x".into() }.kind(),
        ErrorKind::InvalidInput
    );
}

#[test]
fn the_tax_errors_are_wrong_input() {
    let date = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    for error in [
        QuoteError::InvalidTaxRate { value: 0 },
        QuoteError::InvalidTaxSystem,
        QuoteError::UnknownProvince { code: "ZZ".into() },
        QuoteError::RatesDateBeforeCurrent {
            date,
            current: date,
        },
    ] {
        assert_eq!(error.kind(), ErrorKind::InvalidInput);
    }
}

#[test]
fn a_location_off_the_map_is_wrong_input() {
    assert_eq!(QuoteError::InvalidLocation.kind(), ErrorKind::InvalidInput);
}

#[test]
fn every_error_has_its_own_code() {
    let day = chrono::NaiveDate::from_ymd_opt(2026, 11, 16).unwrap();
    let codes = [
        QuoteError::BadQuantity {
            description: String::new(),
        }
        .code(),
        QuoteError::NegativePrice {
            description: String::new(),
        }
        .code(),
        QuoteError::CreditTooLarge {
            credit_cents: 0,
            subtotal_cents: 0,
        }
        .code(),
        QuoteError::LineTooLarge {
            description: String::new(),
        }
        .code(),
        QuoteError::DescriptionTooLong { max: 1 }.code(),
        QuoteError::TooManyLines { max: 1 }.code(),
        QuoteError::InvalidLineKind.code(),
        QuoteError::MissingField { field: "x" }.code(),
        QuoteError::InvalidPhone {
            input: String::new(),
        }
        .code(),
        QuoteError::InvalidEmail {
            input: String::new(),
        }
        .code(),
        QuoteError::InvalidPostalCode {
            input: String::new(),
        }
        .code(),
        QuoteError::InvalidQuoteNumber {
            input: String::new(),
        }
        .code(),
        QuoteError::QuoteNumbersExhausted { year: 2026 }.code(),
        QuoteError::MissingIdentification.code(),
        QuoteError::InvalidMeterReading { reading: 0.0 }.code(),
        QuoteError::LookupFailed { provider: "x" }.code(),
        QuoteError::DatabaseUnavailable.code(),
        QuoteError::MissingSetting { name: "x" }.code(),
        QuoteError::InvalidSetting { name: "x" }.code(),
        QuoteError::ExpiryBeforeQuoteDate {
            date: day,
            quote_date: day,
        }
        .code(),
        QuoteError::PromiseBeforeQuoteDate {
            date: day,
            quote_date: day,
        }
        .code(),
        QuoteError::AlreadyDecided { status: "x" }.code(),
        QuoteError::QuoteExpired { valid_until: day }.code(),
        QuoteError::NotRejected.code(),
        QuoteError::RejectionReasonTooLong { max: 1 }.code(),
        QuoteError::AlreadyAttached { what: "x" }.code(),
        QuoteError::EquipmentNeedsCustomer.code(),
        QuoteError::EquipmentNotOwnedByCustomer.code(),
        QuoteError::QuoteIncomplete { missing: "x" }.code(),
        QuoteError::InvalidLocation.code(),
        QuoteError::BranchInactive.code(),
        QuoteError::DatabaseSetupFailed.code(),
        QuoteError::InvalidStatus { value: "x".into() }.code(),
        QuoteError::UnknownBranch.code(),
        QuoteError::UnknownCustomer.code(),
        QuoteError::CustomerAlreadySaved.code(),
        QuoteError::CustomerNotSaved.code(),
        QuoteError::CustomerInactive.code(),
        QuoteError::InvalidMeterType { value: "x".into() }.code(),
        QuoteError::InvalidEquipmentType { value: "x".into() }.code(),
        QuoteError::InvalidEquipmentSource.code(),
        QuoteError::UnknownEquipment.code(),
        QuoteError::EquipmentAlreadySaved.code(),
        QuoteError::EquipmentNotSaved.code(),
        QuoteError::EquipmentInactive.code(),
        QuoteError::DuplicateSerialNumber.code(),
        QuoteError::UnknownQuote.code(),
        QuoteError::QuoteAlreadySaved.code(),
        QuoteError::QuoteNotSaved.code(),
        QuoteError::QuoteChangedMeanwhile.code(),
        QuoteError::StoredQuoteDamaged.code(),
        QuoteError::UnknownRepairOrder.code(),
        QuoteError::RepairOrderNotSaved.code(),
        QuoteError::QuoteNotAccepted.code(),
        QuoteError::RepairOrderExists.code(),
        QuoteError::MeterReadingRequired.code(),
        QuoteError::RepairOrderNotOpen.code(),
        QuoteError::RepairOrderNotInProgress.code(),
        QuoteError::RepairOrderClosed.code(),
        QuoteError::DueDateBeforeOpened.code(),
        QuoteError::CancellationReasonTooLong { max: 500 }.code(),
        QuoteError::RepairOrderChangedMeanwhile.code(),
        QuoteError::StoredRepairOrderDamaged.code(),
        QuoteError::RepairOrderNumbersExhausted { year: 2026 }.code(),
        QuoteError::InvalidRepairOrderNumber { input: "x".into() }.code(),
        QuoteError::ServerStartFailed.code(),
        QuoteError::DuplicateBranchNumber { number: "x".into() }.code(),
        QuoteError::BranchAlreadySaved.code(),
        QuoteError::BranchNotSaved.code(),
        QuoteError::InvalidTaxRate { value: 0 }.code(),
        QuoteError::InvalidTaxSystem.code(),
        QuoteError::UnknownProvince { code: "x".into() }.code(),
        QuoteError::RatesDateBeforeCurrent {
            date: chrono::NaiveDate::MIN,
            current: chrono::NaiveDate::MIN,
        }
        .code(),
    ];
    let mut unique = codes.to_vec();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), codes.len());
}

#[test]
fn an_error_works_as_a_standard_error() {
    // thiserror also makes QuoteError a std::error::Error, so it can be
    // boxed, chained and used with `?` in code that doesn't know QuoteError.
    let error: Box<dyn std::error::Error> = Box::new(QuoteError::MissingIdentification);
    assert!(error.to_string().contains("serial number"));
}

#[test]
fn a_failed_lookup_tells_the_person_to_enter_the_details_by_hand() {
    let error = QuoteError::LookupFailed {
        provider: "NHTSA vPIC",
    };
    assert_eq!(
        error.to_string(),
        "Could not look up the details with NHTSA vPIC. Please enter them by hand."
    );
}

#[test]
fn a_failed_lookup_is_a_service_problem_not_a_typo() {
    let error = QuoteError::LookupFailed { provider: "x" };
    assert_eq!(error.kind(), ErrorKind::ServiceUnavailable);
    assert_eq!(error.code(), "LOOKUP_FAILED");
    assert_eq!(error.exit_code(), 4);
}

#[test]
fn an_unreachable_database_is_a_service_problem() {
    let error = QuoteError::DatabaseUnavailable;
    assert_eq!(
        error.to_string(),
        "Could not reach the database. Please try again later."
    );
    assert_eq!(error.kind(), ErrorKind::ServiceUnavailable);
    assert_eq!(error.code(), "DATABASE_UNAVAILABLE");
    assert_eq!(error.exit_code(), 4);
}

#[test]
fn a_wrong_setup_is_a_configuration_problem() {
    let missing = QuoteError::MissingSetting {
        name: "POSTGRES_USER",
    };
    assert_eq!(
        missing.to_string(),
        "The setting POSTGRES_USER is missing. Copy .env.example to .env and fill it in."
    );
    assert_eq!(missing.kind(), ErrorKind::Configuration);
    assert_eq!(missing.code(), "MISSING_SETTING");
    assert_eq!(missing.exit_code(), 5);

    let invalid = QuoteError::InvalidSetting {
        name: "POSTGRES_PORT",
    };
    assert_eq!(
        invalid.to_string(),
        "The setting POSTGRES_PORT is not valid."
    );
    assert_eq!(invalid.kind(), ErrorKind::Configuration);
    assert_eq!(invalid.code(), "INVALID_SETTING");
    assert_eq!(invalid.exit_code(), 5);
}

#[test]
fn a_date_before_the_quote_date_is_wrong_input() {
    let date = chrono::NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
    let quote_date = chrono::NaiveDate::from_ymd_opt(2026, 11, 16).unwrap();

    let expiry = QuoteError::ExpiryBeforeQuoteDate { date, quote_date };
    assert_eq!(
        expiry.to_string(),
        "The expiry date can't be before the quote date."
    );
    assert_eq!(expiry.kind(), ErrorKind::InvalidInput);
    assert_eq!(expiry.code(), "EXPIRY_BEFORE_QUOTE_DATE");
    assert_eq!(expiry.exit_code(), 2);

    let promise = QuoteError::PromiseBeforeQuoteDate { date, quote_date };
    assert_eq!(
        promise.to_string(),
        "The promise date can't be before the quote date."
    );
    assert_eq!(promise.kind(), ErrorKind::InvalidInput);
    assert_eq!(promise.code(), "PROMISE_BEFORE_QUOTE_DATE");
    assert_eq!(promise.exit_code(), 2);
}

#[test]
fn deciding_a_quote_wrongly_is_a_business_rule_problem() {
    let day = chrono::NaiveDate::from_ymd_opt(2026, 12, 16).unwrap();

    let decided = QuoteError::AlreadyDecided { status: "accepted" };
    assert_eq!(decided.to_string(), "This quote is already accepted.");
    assert_eq!(decided.kind(), ErrorKind::BusinessRule);
    assert_eq!(decided.code(), "ALREADY_DECIDED");
    assert_eq!(decided.exit_code(), 3);

    let expired = QuoteError::QuoteExpired { valid_until: day };
    assert_eq!(
        expired.to_string(),
        "This quote was only valid until 2026-12-16. Extend its expiry date before it can be accepted."
    );
    assert_eq!(expired.kind(), ErrorKind::BusinessRule);
    assert_eq!(expired.code(), "QUOTE_EXPIRED");
    assert_eq!(expired.exit_code(), 3);

    let not_rejected = QuoteError::NotRejected;
    assert_eq!(
        not_rejected.to_string(),
        "Only a rejected quote can be reopened."
    );
    assert_eq!(not_rejected.kind(), ErrorKind::BusinessRule);
    assert_eq!(not_rejected.code(), "NOT_REJECTED");
}

#[test]
fn a_reason_that_is_too_long_is_wrong_input() {
    let error = QuoteError::RejectionReasonTooLong { max: 500 };
    assert_eq!(
        error.to_string(),
        "The rejection reason can be at most 500 characters."
    );
    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert_eq!(error.code(), "REJECTION_REASON_TOO_LONG");
    assert_eq!(error.exit_code(), 2);
}

#[test]
fn the_line_limit_errors_have_the_right_kind() {
    assert_eq!(
        QuoteError::TooManyLines { max: 1 }.kind(),
        ErrorKind::BusinessRule
    );
    for error in [
        QuoteError::LineTooLarge {
            description: String::new(),
        },
        QuoteError::DescriptionTooLong { max: 1 },
        QuoteError::InvalidLineKind,
    ] {
        assert_eq!(error.kind(), ErrorKind::InvalidInput);
    }
}

// ===== no message repeats what the caller sent (api-rules 10, Day 58) =====

#[test]
fn no_error_message_repeats_the_value_that_was_refused() {
    let marker = "ZZmarker9137";
    let day = chrono::NaiveDate::from_ymd_opt(2026, 11, 16).unwrap();
    let errors = vec![
        QuoteError::BadQuantity {
            description: marker.into(),
        },
        QuoteError::NegativePrice {
            description: marker.into(),
        },
        QuoteError::LineTooLarge {
            description: marker.into(),
        },
        QuoteError::InvalidPhone {
            input: marker.into(),
        },
        QuoteError::InvalidEmail {
            input: marker.into(),
        },
        QuoteError::InvalidPostalCode {
            input: marker.into(),
        },
        QuoteError::InvalidQuoteNumber {
            input: marker.into(),
        },
        QuoteError::InvalidStatus {
            value: marker.into(),
        },
        QuoteError::UnknownProvince {
            code: marker.into(),
        },
        QuoteError::DuplicateBranchNumber {
            number: marker.into(),
        },
        QuoteError::InvalidMeterType {
            value: marker.into(),
        },
        QuoteError::InvalidEquipmentType {
            value: marker.into(),
        },
        QuoteError::InvalidRepairOrderNumber {
            input: marker.into(),
        },
    ];
    for error in errors {
        let message = error.to_string();
        assert!(
            !message.contains(marker),
            "{error:?} repeats the value: {message}"
        );
    }
    // numbers and dates too
    let reading = QuoteError::InvalidMeterReading { reading: -9137.5 }.to_string();
    assert!(!reading.contains("9137"), "{reading}");
    let early = day.pred_opt().unwrap();
    for message in [
        QuoteError::ExpiryBeforeQuoteDate {
            date: early,
            quote_date: day,
        }
        .to_string(),
        QuoteError::PromiseBeforeQuoteDate {
            date: early,
            quote_date: day,
        }
        .to_string(),
    ] {
        assert!(!message.contains("2026"), "{message}");
    }
}
