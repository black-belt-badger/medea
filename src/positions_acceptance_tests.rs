use super::present_positions as present_positions_with_options;
use crate::cli::ReadFormat;
use std::fs;

fn present_positions(response: &[u8], output: Option<&std::path::Path>) -> Result<String, String> {
    let (payload, findings) =
        present_positions_with_options(response, output, ReadFormat::Text, false)?;
    assert!(findings.is_empty());
    String::from_utf8(payload).map_err(|_| "Invalid positions output.".to_owned())
}

// Same synthetic portfolio and exact decimal spellings as Kirke's acceptance
// checks. Both preserve the original provider JSON without reserialization.
pub(super) const POSITIONS: &[u8] = br#"[{"asset_class":"crypto","asset_id":"private-position-id","asset_marginable":false,"avg_entry_price":"67123.0000000000000000000001","change_today":"0","cost_basis":"68.00123456789012345678","current_price":"67291.12","exchange":"CRYPTO","lastday_price":"66500.50","market_value":"67.123456789012345","qty":"0.000997498123456789012345","side":"long","symbol":"BTCUSD","unrealized_intraday_pl":"0","unrealized_intraday_plpc":"0","unrealized_pl":"-0.000000000001","unrealized_plpc":"-0.0000000000123456789","qty_available":null,"future_provider_property":{"decimal":"1.00000000000000000000001"}}]"#;

#[test]
fn positions_summary_keeps_all_decimal_digits_and_omits_private_ids() {
    let summary = present_positions(POSITIONS, None).unwrap();
    assert!(summary.starts_with("Medea — Alpaca paper positions\nPositions: 1\n"));
    for expected in [
        "BTCUSD (crypto, long)\n",
        "  Quantity: 0.000997498123456789012345\n",
        "  Market value: 67.123456789012345\n",
        "  Unrealized P/L: -0.000000000001\n",
    ] {
        assert!(summary.contains(expected), "missing {expected}");
    }
    assert!(!summary.contains("private-position-id"));
    assert!(!summary.contains("future_provider_property"));
}

#[test]
fn empty_portfolio_is_a_success_in_summary_and_private_json() {
    assert_eq!(
        present_positions(b"[]", None).unwrap(),
        "Medea — Alpaca paper positions\nNo open positions.\n"
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("positions.json");
    let notice = present_positions(b"[]", Some(&path)).unwrap();
    assert!(notice.contains("Saved complete positions JSON to "));
    assert_eq!(fs::read(path).unwrap(), b"[]");
}

#[test]
fn export_keeps_provider_bytes_exactly_without_echoing_sensitive_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("positions.json");
    let notice = present_positions(POSITIONS, Some(&path)).unwrap();
    assert_eq!(fs::read(path).unwrap(), POSITIONS);
    assert!(notice.starts_with("Saved complete positions JSON to "));
    for private in [
        "private-position-id",
        "67123.0000000000000000000001",
        "future_provider_property",
    ] {
        assert!(!notice.contains(private));
    }
}

#[test]
fn malformed_missing_required_and_incorrect_types_never_replace_a_prior_export() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("positions.json");
    fs::write(&path, b"previously saved data").unwrap();
    for invalid in [
        &b""[..],
        &b"null"[..],
        &b"{}"[..],
        &b"[null]"[..],
        &b"[{}]"[..],
        &b"[{"[..],
        &br#"[{"asset_class":"crypto"}]"#[..],
    ] {
        let error = present_positions(invalid, Some(&path)).unwrap_err();
        assert_eq!(error, "Invalid Alpaca positions response.");
        assert_eq!(fs::read(&path).unwrap(), b"previously saved data");
    }
}

#[test]
fn unwritable_destination_fails_without_printing_provider_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing-directory").join("positions.json");
    let error = present_positions(POSITIONS, Some(&path)).unwrap_err();
    assert!(error.starts_with("Could not save positions JSON:"));
    assert!(!error.contains("private-position-id"));
    assert!(!path.exists());
}

#[cfg(unix)]
#[test]
fn private_export_replaces_public_file_with_owner_only_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("positions.json");
    fs::write(&path, b"old portfolio").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    present_positions(POSITIONS, Some(&path)).unwrap();
    assert_eq!(fs::read(&path).unwrap(), POSITIONS);
    let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}
