// tests/api/settings.rs: the API's address and key settings

use super::support::KEY;
use quoto::api::ApiSettings;
use quoto::*;

fn settings(pairs: &[(&str, &str)]) -> AppResult<ApiSettings> {
    ApiSettings::from_lookup(|name| {
        pairs
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| v.to_string())
    })
}

#[test]
fn the_api_needs_a_key_and_has_no_default_one() {
    assert_eq!(
        settings(&[]).unwrap_err(),
        AppError::MissingSetting { name: "API_KEY" }
    );
    assert_eq!(
        settings(&[("API_KEY", "   ")]).unwrap_err(),
        AppError::MissingSetting { name: "API_KEY" }
    );
}

#[test]
fn a_short_key_is_refused() {
    assert_eq!(
        settings(&[("API_KEY", "short-key")]).unwrap_err(),
        AppError::InvalidSetting { name: "API_KEY" }
    );
    assert!(settings(&[("API_KEY", KEY)]).is_ok());
}

#[test]
fn it_listens_on_this_computer_port_3000_unless_told_otherwise() {
    let default = settings(&[("API_KEY", KEY)]).unwrap();
    assert_eq!(default.address().to_string(), "127.0.0.1:3000");
    let other = settings(&[("API_KEY", KEY), ("API_PORT", "8080")]).unwrap();
    assert_eq!(other.address().to_string(), "127.0.0.1:8080");
}

#[test]
fn it_refuses_to_listen_on_the_network_or_on_nonsense() {
    for (name, value) in [
        ("API_HOST", "0.0.0.0"),
        ("API_HOST", "192.168.1.5"),
        ("API_HOST", "localhost"),
        ("API_PORT", "http"),
        ("API_PORT", "70000"),
    ] {
        assert_eq!(
            settings(&[("API_KEY", KEY), (name, value)]).unwrap_err(),
            AppError::InvalidSetting { name },
            "{name}={value}"
        );
    }
}

#[test]
fn the_key_is_never_shown_when_the_settings_are_printed() {
    let text = format!("{:?}", settings(&[("API_KEY", KEY)]).unwrap());
    assert!(!text.contains(KEY));
}
