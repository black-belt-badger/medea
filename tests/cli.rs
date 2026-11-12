use std::process::Command;

fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_medea"))
        .args(args)
        // Never read a developer's real paper credentials in offline tests.
        .env_remove("APCA_API_KEY_ID")
        .env_remove("APCA_API_SECRET_KEY")
        .output()
        .expect("Medea binary should be runnable")
}

#[test]
fn help_lists_account_command() {
    let result = cli(&["--help"]);
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("account"));
}

#[test]
fn help_lists_positions_command() {
    let result = cli(&["--help"]);
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("positions"));
}

#[test]
fn positions_help_lists_output_option() {
    let result = cli(&["positions", "--help"]);
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("--out"));
}

#[test]
fn missing_positions_credentials_produce_clear_error() {
    let result = cli(&["positions"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("Missing environment variable: APCA_API_KEY_ID"));
    assert!(result.stdout.is_empty());
}

#[test]
fn missing_positions_output_path_is_invalid_cli() {
    let result = cli(&["positions", "--out"]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("--out"));
}

#[test]
fn account_help_lists_output_option() {
    let result = cli(&["account", "--help"]);
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("--out"));
}

#[test]
fn missing_credentials_produce_clear_error() {
    let result = cli(&["account"]);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("Missing environment variable: APCA_API_KEY_ID"));
    assert!(result.stdout.is_empty());
}

#[test]
fn missing_output_path_is_invalid_cli() {
    let result = cli(&["account", "--out"]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("--out"));
}
