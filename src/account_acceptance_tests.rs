use super::present_account as present_account_with_options;
use crate::cli::ReadFormat;
use std::fs;

// Preserve existing acceptance scenarios while exercising the expanded boundary.
fn present_account(response: &[u8], output: Option<&std::path::Path>) -> Result<String, String> {
    let (payload, findings) =
        present_account_with_options(response, output, ReadFormat::Text, false)?;
    assert!(findings.is_empty());
    String::from_utf8(payload).map_err(|_| "Invalid account output.".to_owned())
}

const ACCOUNT: &[u8] = br#"{
  "id": "private-account-uuid",
  "account_number": "PAPER-001",
  "status": "ACTIVE",
  "crypto_status": "PAPER_ONLY",
  "cash": "-12.345678901234567890123456789",
  "equity": "0",
  "trading_blocked": false,
  "options_trading_level": 8,
  "future_property": {"provider_precision": "1.000000000000000000000001"}
}"#;

#[test]
fn summary_keeps_full_precision_and_shows_independent_account_statuses() {
    let summary = present_account(ACCOUNT, None).unwrap();
    assert!(summary.contains("Status: ACTIVE\n"));
    assert!(summary.contains("Crypto status: PAPER_ONLY\n"));
    assert!(summary.contains("Cash: -12.345678901234567890123456789\n"));
    assert!(summary.contains("Equity: 0\n"));
    assert!(summary.contains("Trading blocked: no\n"));
    assert!(!summary.contains("private-account-uuid"));
    assert!(!summary.contains("future_property"));
}

#[test]
fn export_preserves_original_bytes_but_never_prints_account_values() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("account.json");
    let message = present_account(ACCOUNT, Some(&destination)).unwrap();
    assert_eq!(fs::read(&destination).unwrap(), ACCOUNT);
    assert!(message.contains("Saved complete account JSON to"));
    for sensitive in [
        "private-account-uuid",
        "PAPER-001",
        "-12.345678901234567890123456789",
    ] {
        assert!(!message.contains(sensitive));
    }
}

#[test]
fn malformed_empty_and_wrongly_typed_responses_never_replace_existing_export() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("account.json");
    let original = b"previously exported account";
    fs::write(&destination, original).unwrap();
    for response in [
        &b""[..],
        &b"null"[..],
        &b"[]"[..],
        &b"{}"[..],
        &br#"{"id":"private","status":null}"#[..],
        &br#"{"id":"private","status":"ACTIVE","cash":0.5}"#[..],
    ] {
        let error = present_account(response, Some(&destination)).unwrap_err();
        assert_eq!(error, "Invalid Alpaca account response.");
        assert_eq!(fs::read(&destination).unwrap(), original);
    }
}

#[test]
fn failed_export_returns_error_without_disclosing_response() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("missing").join("account.json");
    let error = present_account(ACCOUNT, Some(&destination)).unwrap_err();
    assert!(error.starts_with("Could not save account JSON:"));
    assert!(!error.contains("private-account-uuid"));
    assert!(!error.contains("PAPER-001"));
    assert!(!destination.exists());
}

#[cfg(unix)]
#[test]
fn exported_account_replaces_a_public_file_with_owner_only_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("account.json");
    fs::write(&destination, b"old account").unwrap();
    fs::set_permissions(&destination, fs::Permissions::from_mode(0o644)).unwrap();

    present_account(ACCOUNT, Some(&destination)).unwrap();

    assert_eq!(fs::read(&destination).unwrap(), ACCOUNT);
    let mode = fs::metadata(&destination).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}
