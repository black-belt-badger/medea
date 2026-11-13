use super::present_assets;
use std::fs;

const ASSETS: &[u8] = br#"[
    {
        "id": "sensitive-internal-asset-id",
        "symbol": "BTC/USD",
        "name": "Bitcoin",
        "class": "crypto",
        "exchange": "CRYPTO",
        "status": "active",
        "tradable": true,
        "marginable": false,
        "shortable": false,
        "fractionable": true,
        "min_order_size": "0.000000000000000000000000001",
        "maintenance_margin_requirement": 0.300000000000000000000000000001,
        "attributes": ["future_asset_attribute"],
        "additional_provider_field": {"nested_precision": "1.00000000000000000000000001"}
    },
    {
        "id": "future-asset-id",
        "symbol": "NEW",
        "name": "New provider asset class",
        "class": "future_class",
        "exchange": "FUTURE_EXCHANGE",
        "status": "future_status",
        "tradable": false,
        "marginable": false,
        "shortable": false,
        "fractionable": false
    }
]"#;

#[test]
fn summary_is_human_readable_and_does_not_print_raw_provider_metadata() {
    let summary = present_assets(ASSETS, None).unwrap();
    for expected in [
        "Medea — Alpaca paper assets\n",
        "Assets: 2\n",
        "BTC/USD (crypto, active)\n",
        "  Name: Bitcoin\n",
        "  Tradable: yes\n",
        "  Fractionable: yes\n",
        "NEW (future_class, future_status)\n",
        "  Tradable: no\n",
        "  Fractionable: no\n",
    ] {
        assert!(summary.contains(expected), "Missing {expected}");
    }
    assert!(!summary.contains("sensitive-internal-asset-id"));
    assert!(!summary.contains("additional_provider_field"));
}

#[test]
fn empty_asset_list_is_valid_for_human_and_json_output() {
    assert_eq!(
        present_assets(b"[]", None).unwrap(),
        "Medea — Alpaca paper assets\nNo matching assets.\n"
    );
    let dir = tempfile::tempdir().unwrap();
    let destination = dir.path().join("assets.json");
    let success = present_assets(b"[]", Some(&destination)).unwrap();
    assert!(success.contains("Saved complete assets JSON to"));
    assert_eq!(fs::read(&destination).unwrap(), b"[]");
}

#[test]
fn raw_export_preserves_full_precision_future_values_and_original_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let destination = dir.path().join("assets.json");
    let notice = present_assets(ASSETS, Some(&destination)).unwrap();
    assert_eq!(fs::read(&destination).unwrap(), ASSETS);
    assert!(notice.contains("Saved complete assets JSON to"));
    for private in [
        "sensitive-internal-asset-id",
        "future-asset-id",
        "nested_precision",
    ] {
        assert!(!notice.contains(private));
    }
}

#[test]
fn invalid_or_incomplete_api_responses_cannot_overwrite_existing_export() {
    let dir = tempfile::tempdir().unwrap();
    let destination = dir.path().join("assets.json");
    let old = b"previous output";
    fs::write(&destination, old).unwrap();

    for response in [
        &b""[..],
        &b"null"[..],
        &b"{}"[..],
        &b"[null]"[..],
        &b"[{"[..],
        &br#"[{"symbol":"BTC/USD"}]"#[..],
        &br#"[{"id":"x","symbol":"BTC/USD","name":"Bitcoin","class":"crypto","exchange":"CRYPTO","status":"active","tradable":true,"marginable":false,"shortable":false,"fractionable":null}]"#[..],
        &br#"[{"id":"x","symbol":"BTC/USD","name":"Bitcoin","class":"crypto","exchange":"CRYPTO","status":"active","tradable":true,"marginable":false,"shortable":false,"fractionable":true,"price_increment":0.1}]"#[..],
    ] {
        let error = present_assets(response, Some(&destination)).unwrap_err();
        assert_eq!(error, "Invalid Alpaca assets response.");
        assert_eq!(fs::read(&destination).unwrap(), old);
    }
}

#[test]
fn failed_export_never_reports_success_or_echoes_provider_data() {
    let dir = tempfile::tempdir().unwrap();
    let destination = dir.path().join("missing-dir").join("assets.json");
    let error = present_assets(ASSETS, Some(&destination)).unwrap_err();
    assert!(error.starts_with("Could not save assets JSON:"));
    assert!(!error.contains("sensitive-internal-asset-id"));
    assert!(!destination.exists());
}

#[cfg(unix)]
#[test]
fn private_export_replaces_public_file_with_owner_only_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let destination = dir.path().join("assets.json");
    fs::write(&destination, b"old").unwrap();
    fs::set_permissions(&destination, fs::Permissions::from_mode(0o644)).unwrap();

    present_assets(ASSETS, Some(&destination)).unwrap();

    assert_eq!(fs::read(&destination).unwrap(), ASSETS);
    let mode = fs::metadata(&destination).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}
