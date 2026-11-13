use super::present_order_get;
use crate::cli::OrderId;
use crate::client::ClientError;
use std::fs;

// Identical fabricated Alpaca source fixtures as Kirke's acceptance tests.
const EQUITY: &[u8] = include_bytes!("../tests/fixtures/orders/order-equity.json");
const CRYPTO: &[u8] = include_bytes!("../tests/fixtures/orders/order-crypto.json");
const MLEG: &[u8] = include_bytes!("../tests/fixtures/orders/order-mleg.json");

fn order_id(input: &str) -> OrderId {
    input.parse().unwrap()
}

fn equity_id() -> OrderId {
    order_id("11111111-2222-4333-8444-555555555555")
}

fn crypto_id() -> OrderId {
    order_id("22222222-3333-4444-8555-666666666666")
}

fn mleg_id() -> OrderId {
    order_id("55555555-6666-4777-8888-999999999999")
}

#[test]
fn equity_and_crypto_summaries_share_order_codec_and_all_decimal_digits() {
    let equity = present_order_get(&equity_id(), Ok(EQUITY.to_vec()), None).unwrap();
    assert!(equity.starts_with("Medea — Alpaca paper order\n"));
    assert!(equity.contains("  Quantity: 5.123456789012345678901\n"));
    assert!(equity.contains("  Filled quantity: 2.000000000000000001\n"));

    let crypto = present_order_get(&crypto_id(), Ok(CRYPTO.to_vec()), None).unwrap();
    assert!(crypto.contains("  Notional: 25.1234567890123456789012345\n"));
}

#[test]
fn nested_options_render_exact_leg_ratios() {
    let summary = present_order_get(&mleg_id(), Ok(MLEG.to_vec()), None).unwrap();
    assert!(summary.contains("  Legs: 2\n"));
    assert!(summary.contains("ratio 1.000000000000000001"));
    assert!(summary.contains("ratio 2.000000000000000002"));
}

#[cfg(unix)]
#[test]
fn valid_lookup_exports_byte_exact_provider_json_and_owner_only_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("order.json");
    fs::write(&path, b"old account-sensitive data").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();

    let message = present_order_get(&equity_id(), Ok(EQUITY.to_vec()), Some(&path)).unwrap();
    assert_eq!(
        message,
        format!("Saved complete order JSON to {}\n", path.display())
    );
    assert_eq!(fs::read(&path).unwrap(), EQUITY);
    assert!(
        EQUITY
            .windows("provider_future_decimal".len())
            .any(|w| w == b"provider_future_decimal")
    );
    let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

#[test]
fn invalid_missing_mismatched_or_unavailable_orders_cannot_clobber_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("order.json");
    fs::write(&path, b"prior-private-order").unwrap();

    for response in [
        Err(ClientError::UnexpectedStatus(404)),
        Err(ClientError::UnexpectedStatus(401)),
        Err(ClientError::UnexpectedStatus(429)),
        Err(ClientError::ConnectionFailed),
        Ok(CRYPTO.to_vec()),
        Ok(b"{}".to_vec()),
        Ok(b"[]".to_vec()),
        Ok(br#"{"id":"11111111-2222-4333-8444-555555555555","status":true}"#.to_vec()),
    ] {
        let error = present_order_get(&equity_id(), response, Some(&path)).unwrap_err();
        assert!(!error.contains("prior-private-order"));
        assert!(!error.contains("SYNTHETIC-EQUITY-ORDER-1"));
        assert_eq!(fs::read(&path).unwrap(), b"prior-private-order");
    }
}

#[test]
fn error_categories_stay_sanitized_and_distinct_at_the_full_command_boundary() {
    assert_eq!(
        present_order_get(&equity_id(), Err(ClientError::UnexpectedStatus(404)), None).unwrap_err(),
        "Alpaca paper order not found (HTTP 404)."
    );
    assert_eq!(
        present_order_get(&equity_id(), Err(ClientError::UnexpectedStatus(401)), None).unwrap_err(),
        "Alpaca paper API rejected the credentials (HTTP 401)."
    );
    assert_eq!(
        present_order_get(&equity_id(), Ok(CRYPTO.to_vec()), None).unwrap_err(),
        "Invalid Alpaca order response: order ID does not match requested UUID."
    );
}

#[test]
fn failed_destination_never_echoes_order_or_prints_success() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing").join("order.json");
    let error = present_order_get(&equity_id(), Ok(EQUITY.to_vec()), Some(&path)).unwrap_err();
    assert!(error.starts_with("Could not save order JSON:"));
    assert!(!error.contains("SYNTHETIC-EQUITY-ORDER-1"));
    assert!(!path.exists());
}
