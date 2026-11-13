//! Pure order-by-ID response boundary.
//!
//! The HTTP transport remains read-only and paper-only. This function validates
//! that a successful response is a *single* matching order before it may be
//! printed or written to a private file. Never echo response data on failure.

use crate::cli::OrderId;
use crate::client::ClientError;
use crate::orders::{Order, decode_order};

const INVALID_ORDER: &str = "Invalid Alpaca order response (expected one valid order object).";
const MISMATCHED_ID: &str =
    "Invalid Alpaca order response: order ID does not match requested UUID.";

/// Keep the original provider response intact for private output, including
/// future fields and full-precision numeric literals. The UUID check ignores
/// ASCII case but does not rewrite the provider's value.
pub fn decode_order_lookup(
    requested: &OrderId,
    response: Result<Vec<u8>, ClientError>,
) -> Result<(Order, Vec<u8>), String> {
    match response {
        Err(ClientError::OrderNotFound | ClientError::UnexpectedStatus(404)) => {
            Err(ClientError::OrderNotFound.to_string())
        }
        Err(error) => Err(error.to_string()),
        Ok(raw) => {
            let order = decode_order(&raw).map_err(|_| INVALID_ORDER.to_owned())?;
            if !order.id.as_str().eq_ignore_ascii_case(requested.as_str()) {
                return Err(MISMATCHED_ID.to_owned());
            }
            Ok((order, raw))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{INVALID_ORDER, MISMATCHED_ID, decode_order_lookup};
    use crate::cli::OrderId;
    use crate::client::ClientError;
    use crate::order_types::{KnownOrderSide, KnownOrderState, OrderSide, OrderState};

    const EQUITY: &[u8] = include_bytes!("../tests/fixtures/orders/order-equity.json");
    const CRYPTO: &[u8] = include_bytes!("../tests/fixtures/orders/order-crypto.json");
    const MLEG: &[u8] = include_bytes!("../tests/fixtures/orders/order-mleg.json");

    fn id(value: &str) -> OrderId {
        value.parse().unwrap()
    }

    fn equity_id() -> OrderId {
        id("11111111-2222-4333-8444-555555555555")
    }

    fn crypto_id() -> OrderId {
        id("22222222-3333-4444-8555-666666666666")
    }

    fn multi_id() -> OrderId {
        id("55555555-6666-4777-8888-999999999999")
    }

    #[test]
    fn equity_partial_fill_passes_exact_original_bytes_and_typed_values() {
        let raw = EQUITY.to_vec();
        let (order, preserved) = decode_order_lookup(&equity_id(), Ok(raw.clone())).unwrap();
        assert_eq!(order.id.as_str(), equity_id().as_str());
        assert_eq!(
            &order.status,
            &OrderState::Known(KnownOrderState::PartiallyFilled)
        );
        assert_eq!(order.qty.as_deref(), Some("5.123456789012345678901"));
        assert_eq!(order.filled_qty.as_deref(), Some("2.000000000000000001"));
        assert_eq!(preserved, raw);
        assert!(String::from_utf8_lossy(&preserved).contains("provider_future_decimal"));
        assert!(
            String::from_utf8_lossy(&preserved)
                .contains("0.123456789012345678901234567890123456789")
        );
    }

    #[test]
    fn notional_crypto_order_preserves_original_decimal_quantities() {
        let (order, preserved) = decode_order_lookup(&crypto_id(), Ok(CRYPTO.to_vec())).unwrap();
        assert_eq!(order.qty.as_deref(), None);
        assert_eq!(
            order.notional.as_deref(),
            Some("25.1234567890123456789012345")
        );
        assert_eq!(
            order.filled_qty.as_deref(),
            Some("0.000375291823456789012345")
        );
        assert_eq!(preserved, CRYPTO);
    }

    #[test]
    fn typed_options_parent_with_null_side_has_both_nested_ratios() {
        let (order, raw) = decode_order_lookup(&multi_id(), Ok(MLEG.to_vec())).unwrap();
        assert_eq!(order.side.as_ref(), None);
        let legs = order.legs.as_deref().unwrap();
        assert_eq!(legs.len(), 2);
        assert_eq!(
            legs[0].side.as_ref(),
            Some(&OrderSide::Known(KnownOrderSide::Buy))
        );
        assert_eq!(
            legs[1].side.as_ref(),
            Some(&OrderSide::Known(KnownOrderSide::Sell))
        );
        assert_eq!(
            legs[0].details.ratio_qty.as_deref(),
            Some("1.000000000000000001")
        );
        assert_eq!(
            legs[1].details.ratio_qty.as_deref(),
            Some("2.000000000000000002")
        );
        assert_eq!(raw, MLEG);
    }

    #[test]
    fn uuid_matching_is_ascii_case_insensitive_and_does_not_normalize_raw_bytes() {
        let requested = id("ABCDEF12-2222-4333-8444-555555555555");
        let raw = br#"{"id":"abcdef12-2222-4333-8444-555555555555","status":"new"}"#.to_vec();
        let (order, preserved) = decode_order_lookup(&requested, Ok(raw.clone())).unwrap();
        assert_eq!(order.id.as_str(), "abcdef12-2222-4333-8444-555555555555");
        assert_eq!(preserved, raw);
    }

    #[test]
    fn mismatched_successful_response_fails_without_displaying_either_identifier() {
        let error = decode_order_lookup(&equity_id(), Ok(CRYPTO.to_vec())).unwrap_err();
        assert_eq!(error, MISMATCHED_ID);
        assert!(!error.contains("11111111"));
        assert!(!error.contains("22222222"));
        assert!(!error.contains("SYNTHETIC"));
    }

    #[test]
    fn invalid_shapes_types_and_broken_leg_arrays_are_sanitized() {
        let raw_inputs: &[&[u8]] = &[
            b"",
            b"[]",
            b"42",
            br#""PRIVATE_ORDER_RESPONSE""#,
            b"{}",
            br#"{"status":"new","qty":"2"}"#,
            br#"{"id":"","status":"new"}"#,
            br#"{"id":"SECRET_ID","status":true}"#,
            br#"{"id":"11111111-2222-4333-8444-555555555555","status":"new","legs":[null]}"#,
            br#"{"id":"11111111-2222-4333-8444-555555555555","status":"new","qty":12.25}"#,
        ];
        for raw in raw_inputs {
            let error = decode_order_lookup(&equity_id(), Ok(raw.to_vec())).unwrap_err();
            assert_eq!(error, INVALID_ORDER);
            assert!(!error.contains("SECRET"));
            assert!(!error.contains("PRIVATE"));
        }
    }

    #[test]
    fn not_found_is_distinct_from_authentication_and_other_http_errors() {
        for response in [
            ClientError::OrderNotFound,
            ClientError::UnexpectedStatus(404),
        ] {
            let actual = decode_order_lookup(&equity_id(), Err(response)).unwrap_err();
            assert_eq!(actual, "Alpaca paper order not found (HTTP 404).");
        }
        for (error, expected) in [
            (
                ClientError::UnexpectedStatus(401),
                "Alpaca paper API rejected the credentials (HTTP 401).",
            ),
            (
                ClientError::UnexpectedStatus(403),
                "Alpaca paper API returned HTTP 403.",
            ),
            (
                ClientError::UnexpectedStatus(429),
                "Alpaca paper API returned HTTP 429.",
            ),
            (
                ClientError::UnexpectedStatus(503),
                "Alpaca paper API returned HTTP 503.",
            ),
            (
                ClientError::ConnectionFailed,
                "Could not connect to Alpaca's paper API. Check your network connection.",
            ),
            (
                ClientError::InvalidCredentials,
                "Invalid Alpaca credentials in environment.",
            ),
        ] {
            let actual = decode_order_lookup(&equity_id(), Err(error)).unwrap_err();
            assert_eq!(actual, expected);
            assert!(!actual.contains("example-secret"));
        }
    }
}
