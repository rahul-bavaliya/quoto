// tests/database_settings.rs: reading the database settings, and keeping secrets secret

use quoto::*;
use std::collections::HashMap;

const PASSWORD: &str = "pw-for-tests-9f3k2";

/// A pretend environment with every required setting (and nothing else).
fn environment() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("POSTGRES_USER", "quoto"),
        ("POSTGRES_PASSWORD", PASSWORD),
        ("POSTGRES_DB", "quoto_dev"),
    ])
}

fn read(environment: &HashMap<&str, &str>) -> QuoteResult<DatabaseSettings> {
    DatabaseSettings::from_lookup(|name| environment.get(name).map(|value| value.to_string()))
}

// ===== reading =====

#[test]
fn required_settings_are_enough_and_the_rest_have_safe_defaults() {
    let settings = read(&environment()).unwrap();
    assert_eq!(settings.describe(), "localhost:5432/quoto_dev");
}

#[test]
fn host_and_port_can_be_changed() {
    let mut env = environment();
    env.insert("POSTGRES_HOST", "db.internal");
    env.insert("POSTGRES_PORT", "5433");
    assert_eq!(read(&env).unwrap().describe(), "db.internal:5433/quoto_dev");
}

#[test]
fn a_missing_user_password_or_database_stops_the_program() {
    for name in ["POSTGRES_USER", "POSTGRES_PASSWORD", "POSTGRES_DB"] {
        let mut env = environment();
        env.remove(name);
        let error = read(&env).unwrap_err();
        assert_eq!(error, QuoteError::MissingSetting { name });
    }
}

#[test]
fn a_blank_password_counts_as_missing() {
    let mut env = environment();
    env.insert("POSTGRES_PASSWORD", "   ");
    assert_eq!(
        read(&env).unwrap_err(),
        QuoteError::MissingSetting {
            name: "POSTGRES_PASSWORD"
        }
    );
}

#[test]
fn a_port_that_is_not_a_number_is_refused() {
    for bad in ["abc", "-1", "70000", "5432; drop"] {
        let mut env = environment();
        env.insert("POSTGRES_PORT", bad);
        assert_eq!(
            read(&env).unwrap_err(),
            QuoteError::InvalidSetting {
                name: "POSTGRES_PORT"
            }
        );
    }
}

#[test]
fn an_unknown_ssl_mode_is_refused() {
    let mut env = environment();
    env.insert("POSTGRES_SSLMODE", "sometimes");
    assert_eq!(
        read(&env).unwrap_err(),
        QuoteError::InvalidSetting {
            name: "POSTGRES_SSLMODE"
        }
    );
}

#[test]
fn known_ssl_modes_are_accepted() {
    for mode in ["disable", "prefer", "require", "verify-ca", "verify-full"] {
        let mut env = environment();
        env.insert("POSTGRES_SSLMODE", mode);
        assert!(read(&env).is_ok(), "{mode} should be accepted");
    }
}

// ===== secrets stay secret =====

#[test]
fn a_secret_never_shows_itself() {
    let secret = Secret::new(String::from(PASSWORD));
    assert_eq!(secret.to_string(), "[hidden]");
    assert_eq!(format!("{secret:?}"), "Secret([hidden])");
    assert_eq!(secret.expose(), PASSWORD);
}

#[test]
fn the_settings_never_show_the_password() {
    let settings = read(&environment()).unwrap();
    assert!(!format!("{settings:?}").contains(PASSWORD));
    assert!(!settings.describe().contains(PASSWORD));
}

#[test]
fn setting_errors_never_repeat_the_value() {
    let mut env = environment();
    env.insert("POSTGRES_PORT", "secret-looking-value");
    let message = read(&env).unwrap_err().to_string();
    assert!(!message.contains("secret-looking-value"));
}
