use super::*;

use serde_json::{Value, json};

pub(crate) fn sample_args() -> SubmitOrderArgs {
    SubmitOrderArgs {
        symbol: "BTC/USD".parse().unwrap(),
        side: SubmitSide::Buy,
        kind: SubmitKind::Market,
        time_in_force: SubmitTimeInForce::Gtc,
        qty: Some("0.000100001".parse().unwrap()),
        notional: None,
        limit_price: None,
        client_order_id: "experiment-01".parse().unwrap(),
        confirm: false,
        output: None,
    }
}

#[test]
fn supports_one_safe_uppercase_crypto_pair_only() {
    for pair in ["BTC/USD", "ETH/BTC", "SOL/USDT", "A1/B2"] {
        assert!(pair.parse::<SubmitSymbol>().is_ok(), "{pair}");
    }
    for pair in [
        "",
        "BTCUSD",
        "btc/USD",
        "BTC/USD,ETH/USD",
        "BTC/USD,",
        "BTC/USD#fragment",
        "BTC/USD?side=sell",
        "BTC/USD;rm",
        "BTC/USD/USDT",
        "/USD",
        "BTC/",
        "BTC%2FUSD",
        "BTC/U S D",
        "ETH/USD\n",
        "../orders",
    ] {
        assert!(pair.parse::<SubmitSymbol>().is_err(), "{pair}");
    }
}

#[test]
fn keeps_positive_exact_decimal_spelling_and_rejects_unrepresentable_amounts() {
    for value in [
        "1",
        "0.000000001",
        "1.200000000",
        "000.0001",
        "12345678901234567890.123456789",
    ] {
        assert_eq!(value.parse::<SubmitAmount>().unwrap().as_str(), value);
    }
    for value in [
        "",
        "0",
        "0.000000000",
        "-1",
        "+1",
        ".5",
        "1.",
        "1.2345678901",
        "1e-8",
        "NaN",
        "1.2.3",
        "1 0",
        "1\n2",
        "１２",
        "0x1",
        "1;echo",
        "100,23",
    ] {
        assert!(value.parse::<SubmitAmount>().is_err(), "{value}");
    }
    assert!("1".repeat(65).parse::<SubmitAmount>().is_err());
}

#[test]
fn caller_supplied_client_id_is_safe_and_stable() {
    for id in ["experiment-01", "btc_2026:11.13", &"A".repeat(128)] {
        assert_eq!(id.parse::<ClientOrderId>().unwrap().as_str(), id);
    }
    for id in [
        "",
        "a b",
        "id\nsecret",
        "id/path",
        "id?symbol=BTC",
        "id😼",
        "id&command",
    ] {
        assert!(id.parse::<ClientOrderId>().is_err(), "{id}");
    }
    assert!("a".repeat(129).parse::<ClientOrderId>().is_err());
}

#[test]
fn dry_run_and_confirmed_valid_market_orders_share_identical_body() {
    let dry = sample_args();
    let mut confirmed = dry.clone();
    confirmed.confirm = true;
    let preview = dry.prepare().unwrap();
    assert_eq!(
        preview.body().unwrap(),
        confirmed.prepare().unwrap().body().unwrap()
    );
    assert_eq!(preview.client_order_id(), "experiment-01");
    let text = dry_run_message(&preview).unwrap();
    assert!(text.contains("DRY RUN"));
    assert!(text.contains("no credentials loaded"));
    assert!(text.contains("no order submitted"));
    assert!(text.contains("A reused client_order_id does not guarantee exactly-once delivery"));
    assert!(!text.contains("APCA-API-SECRET-KEY"));
    assert!(text.contains(r#""qty":"0.000100001""#));
}

#[test]
fn amount_and_notional_are_mutually_exclusive_and_required() {
    let mut args = sample_args();
    args.qty = None;
    assert_eq!(
        args.prepare().unwrap_err(),
        "Exactly one of --qty or --notional is required"
    );
    args.notional = Some("25.12".parse().unwrap());
    assert!(args.prepare().is_ok());
    args.qty = Some("1.25".parse().unwrap());
    assert_eq!(
        args.prepare().unwrap_err(),
        "--qty and --notional cannot be combined"
    );
}

#[test]
fn limit_requires_quantity_and_limit_price_and_market_forbids_price() {
    let mut args = sample_args();
    args.kind = SubmitKind::Limit;
    assert_eq!(
        args.prepare().unwrap_err(),
        "Limit orders require --limit-price"
    );
    args.limit_price = Some("70000.123456789".parse().unwrap());
    assert!(args.prepare().is_ok());
    args.qty = None;
    args.notional = Some("25".parse().unwrap());
    assert_eq!(
        args.prepare().unwrap_err(),
        "Limit orders require --qty; --notional is supported only for market orders"
    );
    args.kind = SubmitKind::Market;
    args.qty = Some("1".parse().unwrap());
    args.notional = None;
    assert_eq!(
        args.prepare().unwrap_err(),
        "--limit-price is only allowed for limit orders"
    );
}

#[test]
fn usd_notional_rejects_excess_precision_but_non_usd_does_not_round() {
    let mut args = sample_args();
    args.qty = None;
    args.notional = Some("25.123".parse().unwrap());
    assert_eq!(
        args.prepare().unwrap_err(),
        "USD-quoted crypto notional supports at most 2 decimal places"
    );
    args.notional = Some("25.12".parse().unwrap());
    assert!(args.prepare().is_ok());
    args.symbol = "ETH/BTC".parse().unwrap();
    args.notional = Some("0.123456789".parse().unwrap());
    assert!(args.prepare().is_ok());
    assert_eq!(
        serde_json::to_value(args.prepare().unwrap()).unwrap()["notional"],
        "0.123456789"
    );
}

#[test]
fn raw_output_requires_confirm_and_never_writes_during_preview() {
    let mut args = sample_args();
    args.output = Some(PathBuf::from("paper-order.json"));
    assert_eq!(
        args.prepare().unwrap_err(),
        "--out requires --confirm; dry runs do not write response files"
    );
    args.confirm = true;
    assert!(args.prepare().is_ok());
}

#[test]
fn serializes_exact_provider_decimal_strings_and_no_extra_fields() {
    let args = sample_args();
    let value: Value = serde_json::from_slice(&args.prepare().unwrap().body().unwrap()).unwrap();
    assert_eq!(
        value,
        json!({
            "symbol": "BTC/USD",
            "side": "buy",
            "type": "market",
            "time_in_force": "gtc",
            "qty": "0.000100001",
            "client_order_id": "experiment-01"
        })
    );
    assert!(value.get("notional").is_none());
    assert!(value.get("limit_price").is_none());
    assert!(value.get("confirm").is_none());
    assert!(value.get("output").is_none());
}

#[test]
fn writes_only_notional_for_market_and_precise_limit_price_for_limit() {
    let mut args = sample_args();
    args.qty = None;
    args.notional = Some("25.12".parse().unwrap());
    let value: Value = serde_json::from_slice(&args.prepare().unwrap().body().unwrap()).unwrap();
    assert_eq!(value["notional"], "25.12");
    assert!(value.get("qty").is_none());
    assert!(value.get("limit_price").is_none());

    args.notional = None;
    args.qty = Some("0.000000001".parse().unwrap());
    args.kind = SubmitKind::Limit;
    args.side = SubmitSide::Sell;
    args.time_in_force = SubmitTimeInForce::Ioc;
    args.limit_price = Some("80000.100000001".parse().unwrap());
    let value: Value = serde_json::from_slice(&args.prepare().unwrap().body().unwrap()).unwrap();
    assert_eq!(value["qty"], "0.000000001");
    assert_eq!(value["limit_price"], "80000.100000001");
    assert_eq!(value["side"], "sell");
    assert_eq!(value["time_in_force"], "ioc");
    assert_eq!(value["type"], "limit");
}

const MATCHING: &[u8] = br#"{"id":"11111111-2222-4333-8444-555555555555","client_order_id":"experiment-01","symbol":"BTC/USD","status":"new","qty":"0.000100001","future_decimal":0.000000000000001234567890123456789}"#;

#[test]
fn accepts_matching_response_preserving_exact_private_export_bytes() {
    let args = sample_args();
    let (order, raw) =
        decode_submit_result(&args.prepare().unwrap(), Ok(MATCHING.to_vec())).unwrap();
    assert_eq!(order.client_order_id.as_deref(), Some("experiment-01"));
    assert_eq!(order.qty.as_deref(), Some("0.000100001"));
    assert_eq!(raw, MATCHING);
    assert!(
        raw.windows(b"0.000000000000001234567890123456789".len())
            .any(|window| window == b"0.000000000000001234567890123456789")
    );
}

#[test]
fn rejects_mismatched_and_missing_client_ids_without_echoing_provider_body() {
    let args = sample_args();
    for raw in [
        br#"{"id":"secret-id","client_order_id":"SECRET_OTHER","status":"new"}"#.as_slice(),
        br#"{"id":"secret-id","status":"new"}"#.as_slice(),
    ] {
        let error = decode_submit_result(&args.prepare().unwrap(), Ok(raw.to_vec())).unwrap_err();
        assert_eq!(
            error,
            "Submission response client_order_id does not match. Check paper orders before retrying."
        );
        assert!(!error.contains("SECRET"));
        assert!(!error.contains("secret-id"));
    }
}

#[test]
fn rejects_malformed_success_response_as_uncertain_without_exposing_json_details() {
    let args = sample_args();
    for raw in [
        "[]",
        "null",
        "\"PRIVATE_RAW_DATA\"",
        "42",
        "not-json",
        "{}",
        r#"{"id":"abc"}"#,
        r#"{"status":"new"}"#,
        r#"{"id":"abc","status":true}"#,
    ] {
        let error = decode_submit_result(&args.prepare().unwrap(), Ok(raw.as_bytes().to_vec()))
            .unwrap_err();
        assert_eq!(
            error,
            "Invalid paper-order submission response. Check paper orders before retrying."
        );
        assert!(!error.contains("PRIVATE"));
        assert!(!error.contains("abc"));
    }
}

#[test]
fn status_errors_are_actionable_and_uncertain_failures_never_claim_no_order() {
    for (err, expected) in [
        (
            ClientError::UnexpectedStatus(401),
            "Alpaca paper API rejected credentials (HTTP 401). No response body exposed.",
        ),
        (
            ClientError::UnexpectedStatus(403),
            "Alpaca paper order rejected (HTTP 403; check permissions or buying power).",
        ),
        (
            ClientError::UnexpectedStatus(422),
            "Alpaca paper order rejected (HTTP 422; check order parameters and asset rules).",
        ),
        (
            ClientError::UnexpectedStatus(429),
            "Alpaca paper API rate-limited submission (HTTP 429). Check paper orders before retrying.",
        ),
        (
            ClientError::UnexpectedStatus(503),
            "Alpaca paper API returned HTTP 503 during submission. Check paper orders before retrying.",
        ),
        (
            ClientError::UnexpectedStatus(504),
            "Alpaca paper API returned HTTP 504 during submission. Check paper orders before retrying.",
        ),
        (
            ClientError::UnexpectedStatus(400),
            "Alpaca paper order submission returned HTTP 400.",
        ),
    ] {
        assert_eq!(submission_error_message(&err), expected);
        assert_eq!(
            decode_submit_result(&sample_args().prepare().unwrap(), Err(err)).unwrap_err(),
            expected
        );
    }
    let uncertain = submission_error_message(&ClientError::ConnectionFailed);
    assert!(uncertain.contains("result unknown"));
    assert!(uncertain.contains("Check existing paper orders"));
    assert!(!uncertain.contains("order not submitted"));
}
