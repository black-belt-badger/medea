use super::present_orders;
use std::fs;

// Shared with Kirke: fabricated Alpaca orders (equity, crypto, multi-leg,
// bracket, trailing stop, future enums) and long decimal wire spellings.
const ORDERS: &[u8] = include_bytes!("../tests/fixtures/orders/orders-mixed.json");

#[test]
fn summary_renders_six_shared_order_scenarios_with_exact_decimal_strings() {
    let summary = present_orders(ORDERS, None).unwrap();
    assert!(summary.starts_with("Medea — Alpaca paper orders\nOrders: 6\n"));
    for expected in [
        "  Quantity: 5.123456789012345678901\n",
        "  Filled quantity: 2.000000000000000001\n",
        "  Notional: 25.1234567890123456789012345\n",
        "  Legs: 2\n",
    ] {
        assert!(summary.contains(expected), "missing {expected}");
    }
}

#[test]
fn empty_list_succeeds_for_both_summary_and_exact_json_export() {
    assert_eq!(
        present_orders(b"[]", None).unwrap(),
        "Medea — Alpaca paper orders\nNo matching orders.\n"
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("orders.json");
    let notice = present_orders(b"[]", Some(&path)).unwrap();
    assert!(notice.starts_with("Saved complete orders JSON to "));
    assert_eq!(fs::read(&path).unwrap(), b"[]");
}

#[test]
fn private_export_preserves_unknown_fields_and_precise_original_json_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("orders.json");
    let notice = present_orders(ORDERS, Some(&path)).unwrap();
    assert_eq!(fs::read(&path).unwrap(), ORDERS);
    assert!(notice.starts_with("Saved complete orders JSON to "));
    assert!(!notice.contains("SYNTHETIC-EQUITY-ORDER-1"));
    assert!(!notice.contains("provider_future_decimal"));
}

#[test]
fn malformed_or_incomplete_second_order_cannot_overwrite_prior_export() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("orders.json");
    fs::write(&path, b"previous-good-data").unwrap();
    for response in [
        &b""[..],
        &b"null"[..],
        &b"{}"[..],
        &b"[null]"[..],
        &b"[{}]"[..],
        &b"[{"[..],
        &br#"[{"id":"order-one","status":"filled"},{"id":"missing-status"}]"#[..],
        &br#"[{"id":"order-one","status":"filled"},{"id":"other","status":"filled","qty":3.14}]"#[..],
    ] {
        let error = present_orders(response, Some(&path)).unwrap_err();
        assert_eq!(error, "Invalid Alpaca orders response.");
        assert_eq!(fs::read(&path).unwrap(), b"previous-good-data");
    }
}

#[test]
fn failed_export_does_not_echo_provider_data_or_report_success() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("nonexistent").join("orders.json");
    let error = present_orders(ORDERS, Some(&path)).unwrap_err();
    assert!(error.starts_with("Could not save orders JSON:"));
    assert!(!error.contains("SYNTHETIC-EQUITY-ORDER-1"));
    assert!(!path.exists());
}

#[cfg(unix)]
#[test]
fn successful_export_replaces_public_file_with_owner_only_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("orders.json");
    fs::write(&path, b"old").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();

    present_orders(ORDERS, Some(&path)).unwrap();

    assert_eq!(fs::read(&path).unwrap(), ORDERS);
    let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}
