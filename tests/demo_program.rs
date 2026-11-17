// tests/demo_program.rs: runs the real program and checks what comes out

use std::process::Command;

fn run_demo(rust_log: Option<&str>) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_quoto"));
    command.env_remove("RUST_LOG");
    if let Some(level) = rust_log {
        command.env("RUST_LOG", level);
    }
    command.output().expect("the demo program should start")
}

#[test]
fn the_demo_prints_both_quotes_and_exits_cleanly() {
    let output = run_demo(None);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("Total: $136.53"));
    assert!(stdout.contains("Total: $129.15"));
}

#[test]
fn logs_go_to_stderr_and_never_into_the_quote_text() {
    let output = run_demo(None);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stderr.contains("building the demo quotes"));
    assert!(!stdout.contains("INFO"));
}

#[test]
fn the_log_level_can_be_changed_with_rust_log() {
    let quiet = run_demo(Some("error"));
    let chatty = run_demo(Some("debug"));

    assert!(quiet.stderr.is_empty());
    assert!(String::from_utf8_lossy(&chatty.stderr).contains("line added"));
}

#[test]
fn logs_do_not_contain_customer_details() {
    let output = run_demo(Some("debug"));
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!stderr.contains("Jane Smith"));
    // "-555-" is how the sample phones look; a bare "555" can appear in a timestamp by chance.
    assert!(!stderr.contains("-555-"));
    assert!(!stderr.contains("@"));
}

#[test]
fn each_printed_quote_is_logged_with_its_province_and_total() {
    let output = run_demo(None);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(stderr.matches("quote printed").count(), 2);
    assert!(stderr.contains("Saskatchewan"));
    assert!(stderr.contains("13653"));
}
