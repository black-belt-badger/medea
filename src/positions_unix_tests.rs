use super::{present_positions, render_cli_finding};
use crate::cli::ReadFormat;
use crate::diagnostics::Severity;
use crate::positions_acceptance_tests::POSITIONS;
use std::fs;

#[test]
fn default_text_is_backward_compatible_without_findings() {
    let (payload, findings) = present_positions(POSITIONS, None, ReadFormat::Text, false).unwrap();
    let summary = String::from_utf8(payload).unwrap();
    assert!(summary.starts_with("Medea — Alpaca paper positions\n"));
    assert!(summary.contains("  Quantity: 0.000997498123456789012345\n"));
    assert!(findings.is_empty());
}

#[test]
fn exact_json_stdout_preserves_unknown_fields_and_decimals() {
    let (payload, findings) = present_positions(POSITIONS, None, ReadFormat::Json, true).unwrap();
    assert_eq!(payload, POSITIONS);
    assert!(findings.is_empty());
}

#[test]
fn empty_portfolio_is_an_informational_fact_and_json_remains_an_array() {
    let (payload, findings) = present_positions(b"[]", None, ReadFormat::Json, true).unwrap();
    assert_eq!(payload, b"[]");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].code, "POSITIONS_EMPTY");
    assert_eq!(findings[0].severity, Severity::Info);
    assert_eq!(findings[0].path, "positions");
    assert_eq!(
        render_cli_finding(&findings[0]),
        "INFO POSITIONS_EMPTY (positions): Alpaca returned no open positions."
    );
}

#[test]
fn unknown_position_side_is_reported_without_interpreting_provider_value() {
    let payload = String::from_utf8(POSITIONS.to_vec())
        .unwrap()
        .replace("\"side\":\"long\"", "\"side\":\"FUTURE_PRIVATE_SIDE\"");
    let (json, findings) =
        present_positions(payload.as_bytes(), None, ReadFormat::Json, true).unwrap();
    assert_eq!(json, payload.as_bytes());
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Warning);
    assert_eq!(findings[0].code, "POSITION_SIDE_UNKNOWN");
    assert_eq!(findings[0].path, "positions[].side");
    assert!(!render_cli_finding(&findings[0]).contains("FUTURE_PRIVATE_SIDE"));
    assert!(!render_cli_finding(&findings[0]).contains("private-position-id"));
}

#[test]
fn malformed_second_position_is_rejected_before_stdout_and_private_export() {
    let mut broken = POSITIONS.to_vec();
    broken.pop(); // remove closing array bracket
    broken.extend_from_slice(br#",{}]"#);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("positions.json");
    fs::write(&path, b"prior-private-export").unwrap();
    let err = present_positions(&broken, Some(&path), ReadFormat::Json, true).unwrap_err();
    assert_eq!(err, "Invalid Alpaca positions response.");
    assert_eq!(fs::read(path).unwrap(), b"prior-private-export");
}

#[test]
fn private_output_and_legacy_path_notice_remain_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("positions.json");
    let (notice, findings) = present_positions(b"[]", Some(&path), ReadFormat::Json, true).unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"[]");
    assert_eq!(
        String::from_utf8(notice).unwrap(),
        format!("Saved complete positions JSON to {}\n", path.display())
    );
    assert_eq!(findings.len(), 1);
}

#[test]
fn checked_positions_dont_contain_terminal_ansi_codes() {
    let (json, findings) = present_positions(b"[]", None, ReadFormat::Json, true).unwrap();
    assert!(!json.windows(2).any(|w| w == b"\x1b["));
    assert!(
        findings
            .iter()
            .all(|f| !render_cli_finding(f).contains("\x1b["))
    );
}
