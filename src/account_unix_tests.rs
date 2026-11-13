use super::{present_account, render_account_finding, write_account_bytes};
use crate::cli::AccountFormat;
use crate::diagnostics::Severity;
use std::fs;
use std::io::{self, Write};

const BLOCKED_ACCOUNT: &[u8] = br#"{"id":"PRIVATE_ACCOUNT_ID","account_number":"SECRET-0001","status":"ACTIVE","crypto_status":"ACTIVE","trading_blocked":true,"cash":"12.00000000000000000001","future_numeric":0.12345678901234567890123456789}"#;
const LIMITED_ACCOUNT: &[u8] =
    br#"{"id":"PRIVATE_ACCOUNT_ID","status":"LIMITED","crypto_status":"PAPER_ONLY"}"#;

#[test]
fn default_text_output_is_stable_and_check_is_off() {
    let (payload, findings) =
        present_account(BLOCKED_ACCOUNT, None, AccountFormat::Text, false).unwrap();
    let summary = String::from_utf8(payload).unwrap();
    assert!(summary.starts_with("Medea — Alpaca paper account\n"));
    assert!(summary.contains("Trading blocked: yes\n"));
    assert!(findings.is_empty());
}

#[test]
fn json_is_original_provider_bytes_without_lossy_numeric_reserialization() {
    let (payload, findings) =
        present_account(BLOCKED_ACCOUNT, None, AccountFormat::Json, false).unwrap();
    assert_eq!(payload, BLOCKED_ACCOUNT);
    assert!(findings.is_empty());
}

#[test]
fn checked_json_keeps_data_separate_from_stderr_findings() {
    let (payload, findings) =
        present_account(BLOCKED_ACCOUNT, None, AccountFormat::Json, true).unwrap();
    assert_eq!(payload, BLOCKED_ACCOUNT);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Error);
    assert_eq!(
        render_account_finding(&findings[0]),
        "ERROR ACCOUNT_TRADING_BLOCKED (trading_blocked): Alpaca reports trading_blocked=true."
    );
}

#[test]
fn independent_provider_statuses_have_same_order_and_severity_as_kirke() {
    let (payload, findings) =
        present_account(LIMITED_ACCOUNT, None, AccountFormat::Text, true).unwrap();
    assert!(
        String::from_utf8(payload)
            .unwrap()
            .contains("Status: LIMITED\n")
    );
    let codes: Vec<_> = findings
        .iter()
        .map(|f| (f.code, f.severity, f.path))
        .collect();
    assert_eq!(
        codes,
        [
            ("ACCOUNT_STATUS_NOT_ACTIVE", Severity::Warning, "status"),
            (
                "ACCOUNT_TRADING_BLOCKED_UNKNOWN",
                Severity::Info,
                "trading_blocked"
            ),
            (
                "ACCOUNT_CRYPTO_STATUS_NOT_ACTIVE",
                Severity::Info,
                "crypto_status"
            ),
        ]
    );
    assert_eq!(
        render_account_finding(&findings[0]),
        "WARNING ACCOUNT_STATUS_NOT_ACTIVE (status): Alpaca reports an account status other than ACTIVE."
    );
}

#[test]
fn json_and_diagnostics_are_free_of_terminal_escape_codes() {
    let (payload, findings) =
        present_account(LIMITED_ACCOUNT, None, AccountFormat::Json, true).unwrap();
    assert!(!payload.windows(2).any(|bytes| bytes == b"\x1b["));
    for finding in findings {
        assert!(!render_account_finding(&finding).contains("\x1b["));
    }
}

#[test]
fn malformed_response_rejected_before_output_or_findings() {
    for invalid in [
        &b""[..],
        &b"[]"[..],
        &b"null"[..],
        &b"{}"[..],
        &br#"{"id":"secret","status":true}"#[..],
    ] {
        let error = present_account(invalid, None, AccountFormat::Json, true).unwrap_err();
        assert_eq!(error, "Invalid Alpaca account response.");
    }
}

#[test]
fn private_out_is_unchanged_even_when_json_and_check_are_selected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("account.json");
    let (payload, findings) =
        present_account(BLOCKED_ACCOUNT, Some(&path), AccountFormat::Json, true).unwrap();
    assert_eq!(fs::read(path).unwrap(), BLOCKED_ACCOUNT);
    assert!(
        String::from_utf8(payload)
            .unwrap()
            .starts_with("Saved complete account JSON to ")
    );
    assert_eq!(findings.len(), 1);
}

#[test]
fn findings_do_not_include_provider_identifiers_or_balance_values() {
    let (_, findings) = present_account(BLOCKED_ACCOUNT, None, AccountFormat::Json, true).unwrap();
    let combined = findings
        .iter()
        .map(render_account_finding)
        .collect::<Vec<_>>()
        .join("\n");
    for secret in [
        "PRIVATE_ACCOUNT_ID",
        "SECRET-0001",
        "12.00000000000000000001",
    ] {
        assert!(!combined.contains(secret));
    }
}

struct FailingWriter(io::ErrorKind);

impl Write for FailingWriter {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(self.0, "synthetic IO error"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn deliberately_closed_consumer_pipe_is_a_success() {
    let mut pipe = FailingWriter(io::ErrorKind::BrokenPipe);
    assert_eq!(write_account_bytes(&mut pipe, b"data"), Ok(false));
}

#[test]
fn unrelated_stdout_io_error_is_not_swallowed() {
    let mut writer = FailingWriter(io::ErrorKind::PermissionDenied);
    assert_eq!(
        write_account_bytes(&mut writer, b"data"),
        Err("Could not write account output.".to_owned())
    );
}
