// tests/status.rs: the one "is it in use?" status (public API only)

use quoto::*;

#[test]
fn a_new_record_starts_active() {
    assert_eq!(Status::default(), Status::Active);
}

#[test]
fn the_database_words_are_active_and_inactive() {
    assert_eq!(Status::Active.label(), "active");
    assert_eq!(Status::Inactive.label(), "inactive");
    assert_eq!(Status::Inactive.to_string(), "inactive");
}

#[test]
fn a_word_from_the_database_turns_back_into_the_same_status() {
    for status in [Status::Active, Status::Inactive] {
        assert_eq!(Status::from_label(status.label()), Ok(status));
    }
}

#[test]
fn only_the_two_known_words_are_accepted() {
    for word in ["", "Active", "ACTIVE", "in_active", "deleted", " active"] {
        assert_eq!(
            Status::from_label(word),
            Err(AppError::InvalidStatus {
                value: word.to_string()
            }),
            "{word:?}"
        );
    }
}
