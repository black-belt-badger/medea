use super::*;

use std::cell::RefCell;
use std::fs;

use serde_json::json;

const RESPONSE: &str = r#"{
  "trades": {
    "BTC/USD": [{
      "i":123456789012345678901234567890,
      "p":29000.123456789012345678,
      "s":0.000000000000123456789,
      "t":"2026-10-02T00:00:00.123456789Z",
      "tks":"S",
      "future_trade_field":true
    }],
    "ETH/USD": [{
      "i":31455287,
      "p":2200.5,
      "s":3,
      "t":"2026-10-02T00:00:00Z",
      "tks":null
    }]
  },
  "next_page_token":"resume-token",
  "future_page_metadata":42
}"#;

const FIRST: &[u8] = br#"{"trades":{"BTC/USD":[{"i":123456789012345678901234567890,"p":29000.123456789012345678,"s":0.000000000000123456789,"t":"2026-10-02T00:00:00.123456789Z","tks":"S","future_trade_field":true}]},"next_page_token":"token-2","future_page_metadata":42}"#;
const SECOND: &[u8] = br#"{"trades":{"ETH/USD":[{"i":31455287,"p":2200.5,"s":0.2,"t":"2026-10-02T00:00:00.234567891Z","tks":null}]},"next_page_token":null}"#;

fn parsed(bytes: &[u8]) -> CryptoTradesPage {
    decode_page(bytes).unwrap()
}

fn test_args() -> CryptoTradesArgs {
    CryptoTradesArgs {
        symbols: "BTC/USD,ETH/USD".parse().unwrap(),
        location: CryptoLocation::Us,
        start: None,
        end: None,
        limit: 2,
        sort: BarsSort::Asc,
        max_pages: 2,
        page_token: None,
        format: BarsFormat::Json,
        output: None,
    }
}

#[test]
fn decodes_multi_symbol_trades_with_exact_large_integer_and_decimal_values() {
    let page = parsed(RESPONSE.as_bytes());
    assert_eq!(page.trades.len(), 2);
    let btc = &page.trades["BTC/USD"][0];
    assert_eq!(btc.id.to_string(), "123456789012345678901234567890");
    assert_eq!(btc.price.to_string(), "29000.123456789012345678");
    assert_eq!(btc.size.to_string(), "1.23456789e-13");
    assert_eq!(btc.timestamp, "2026-10-02T00:00:00.123456789Z");
    assert_eq!(btc.taker_side.as_deref(), Some("S"));
    assert_eq!(page.next_page_token.as_deref(), Some("resume-token"));
}

#[test]
fn optional_missing_null_and_future_taker_sides_are_accepted() {
    let page = parsed(RESPONSE.as_bytes());
    assert_eq!(page.trades["ETH/USD"][0].taker_side, None);
    let missing =
        parsed(br#"{"trades":{"SOL/USD":[{"i":1,"p":1,"s":0,"t":"2026-10-02T00:00:00Z"}]}}"#);
    assert_eq!(missing.trades["SOL/USD"][0].taker_side, None);
    let future = parsed(
        br#"{"trades":{"SOL/USD":[{"i":1,"p":1,"s":0,"t":"2026-10-02T00:00:00Z","tks":"NEW"}]}}"#,
    );
    assert_eq!(
        future.trades["SOL/USD"][0].taker_side.as_deref(),
        Some("NEW")
    );
}

#[test]
fn accepts_empty_pages_with_missing_null_or_blank_token() {
    for body in [
        r#"{"trades":{}}"#,
        r#"{"trades":{},"next_page_token":null}"#,
        r#"{"trades":{},"next_page_token":""}"#,
    ] {
        let page = parsed(body.as_bytes());
        assert!(page.trades.is_empty());
        assert_eq!(page.next_page_token, None);
    }
}

#[test]
fn rejects_malformed_responses_and_invalid_trade_id_or_numeric_types() {
    for body in [
        "",
        "[]",
        "{}",
        "{broken",
        r#"{"trades":null}"#,
        r#"{"trades":[1,2,3]}"#,
        r#"{"trades":{"BTC/USD":[{"p":1,"s":2,"t":"2026-10-02T00:00:00Z"}]}}"#,
        r#"{"trades":{"BTC/USD":[{"i":1.5,"p":1,"s":2,"t":"2026-10-02T00:00:00Z"}]}}"#,
        r#"{"trades":{"BTC/USD":[{"i":-2,"p":1,"s":2,"t":"2026-10-02T00:00:00Z"}]}}"#,
        r#"{"trades":{"BTC/USD":[{"i":"123","p":1,"s":2,"t":"2026-10-02T00:00:00Z"}]}}"#,
        r#"{"trades":{"BTC/USD":[{"i":1,"p":"one","s":2,"t":"2026-10-02T00:00:00Z"}]}}"#,
        r#"{"trades":{"BTC/USD":[{"i":1,"p":1,"s":"two","t":"2026-10-02T00:00:00Z"}]}}"#,
    ] {
        assert!(decode_page(body.as_bytes()).is_err(), "{body:?}");
    }
}

#[test]
fn complete_json_page_envelope_preserves_future_fields_and_precise_values() {
    let mut output = Vec::new();
    let summary = write_trades_output(&mut output, BarsFormat::Json, 2, None, |token| {
        Ok(if token.is_none() { FIRST } else { SECOND }.to_vec())
    })
    .unwrap();
    assert_eq!(summary.trades, 2);
    assert_eq!(summary.pages, 2);
    assert_eq!(summary.next_page_token, None);
    let value: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(value["pages"][0], parsed(FIRST).original);
    assert_eq!(value["pages"][1], parsed(SECOND).original);
    assert_eq!(value["pages"][0]["future_page_metadata"], json!(42));
    assert_eq!(
        value["pages"][0]["trades"]["BTC/USD"][0]["future_trade_field"],
        true
    );
    assert_eq!(value["next_page_token"], Value::Null);
    assert!(
        String::from_utf8(output)
            .unwrap()
            .contains("29000.123456789012345678")
    );
}

#[test]
fn jsonl_contains_one_symbolized_trade_per_line_without_rounding() {
    let mut output = Vec::new();
    let summary = write_trades_output(&mut output, BarsFormat::Jsonl, 2, None, |token| {
        Ok(if token.is_none() { FIRST } else { SECOND }.to_vec())
    })
    .unwrap();
    assert_eq!(summary.trades, 2);
    let records: Vec<Value> = output
        .split(|byte| *byte == b'\n')
        .filter(|record| !record.is_empty())
        .map(|record| serde_json::from_slice(record).unwrap())
        .collect();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0]["symbol"], "BTC/USD");
    assert_eq!(records[1]["symbol"], "ETH/USD");
    assert_eq!(
        records[0]["i"].to_string(),
        "123456789012345678901234567890"
    );
    assert_eq!(records[0]["t"], "2026-10-02T00:00:00.123456789Z");
    assert_eq!(records[0]["tks"], "S");
    assert_eq!(records[1]["tks"], Value::Null);
    assert_eq!(records[0]["p"].to_string(), "29000.123456789012345678");
}

#[test]
fn empty_page_has_empty_jsonl_and_readable_summary() {
    let body = br#"{"trades":{},"next_page_token":null}"#;
    let mut output = Vec::new();
    let summary = write_trades_output(&mut output, BarsFormat::Jsonl, 1, None, |_| {
        Ok(body.to_vec())
    })
    .unwrap();
    assert!(output.is_empty());
    assert_eq!(
        render_summary(&summary),
        "Medea — Alpaca historical crypto trades\nPages: 1\nTrades: 0\nNo matching trades.\nMore pages available: no\n"
    );
}

#[test]
fn defaults_to_one_page_even_when_continuation_is_present() {
    let calls = RefCell::new(Vec::new());
    let indices = RefCell::new(Vec::new());
    let summary = fetch_trades_pages_with(
        1,
        None,
        |token| {
            calls.borrow_mut().push(token.map(str::to_owned));
            Ok(FIRST.to_vec())
        },
        |index, _| {
            indices.borrow_mut().push(index);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(summary.pages, 1);
    assert_eq!(summary.trades, 1);
    assert_eq!(summary.next_page_token.as_deref(), Some("token-2"));
    assert_eq!(&*calls.borrow(), &[None]);
    assert_eq!(&*indices.borrow(), &[0]);
    assert!(render_summary(&summary).contains("More pages available: yes"));
}

#[test]
fn follows_multi_page_tokens_and_aggregates_symbols_without_retaining_pages() {
    let calls = RefCell::new(Vec::new());
    let indices = RefCell::new(Vec::new());
    let summary = fetch_trades_pages_with(
        5,
        None,
        |token| {
            calls.borrow_mut().push(token.map(str::to_owned));
            match token {
                None => Ok(FIRST.to_vec()),
                Some("token-2") => Ok(SECOND.to_vec()),
                _ => Err(ClientError::UnexpectedStatus(400)),
            }
        },
        |index, page| {
            indices.borrow_mut().push(index);
            assert_eq!(page.trades.len(), 1);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(summary.trades, 2);
    assert_eq!(summary.pages, 2);
    assert_eq!(summary.next_page_token, None);
    assert_eq!(summary.symbols["BTC/USD"], 1);
    assert_eq!(summary.symbols["ETH/USD"], 1);
    assert_eq!(&*calls.borrow(), &[None, Some("token-2".into())]);
    assert_eq!(&*indices.borrow(), &[0, 1]);
}

#[test]
fn uses_supplied_starting_page_token_exactly() {
    let calls = RefCell::new(Vec::new());
    let summary = fetch_trades_pages_with(
        1,
        Some("ABC+/=="),
        |token| {
            calls.borrow_mut().push(token.map(str::to_owned));
            Ok(SECOND.to_vec())
        },
        |_, _| Ok(()),
    )
    .unwrap();
    assert_eq!(summary.trades, 1);
    assert_eq!(&*calls.borrow(), &[Some("ABC+/==".into())]);
}

#[test]
fn rejects_repeated_token_without_unbounded_requests() {
    let mut calls = 0;
    let result = fetch_trades_pages_with(
        20,
        None,
        |_| {
            calls += 1;
            Ok(br#"{"trades":{},"next_page_token":"repeat"}"#.to_vec())
        },
        |_, _| Ok(()),
    );
    assert_eq!(
        result.err().as_deref(),
        Some("Alpaca crypto trades returned a repeated page token")
    );
    assert_eq!(calls, 2);
}

#[test]
fn out_of_range_page_bounds_fail_without_networking() {
    let mut calls = 0;
    for max_pages in [0, 21] {
        let result = fetch_trades_pages_with(
            max_pages,
            None,
            |_| {
                calls += 1;
                Ok(FIRST.to_vec())
            },
            |_, _| Ok(()),
        );
        assert_eq!(
            result.err().as_deref(),
            Some("Maximum pages must be between 1 and 20")
        );
    }
    assert_eq!(calls, 0);
}

#[test]
fn malformed_later_page_aborts_without_following_more_tokens() {
    let mut calls = 0;
    let result = fetch_trades_pages_with(
        3,
        None,
        |_| {
            calls += 1;
            Ok(if calls == 1 {
                FIRST.to_vec()
            } else {
                br#"{"trades":[]}"#.to_vec()
            })
        },
        |_, _| Ok(()),
    );
    assert_eq!(
        result.err().as_deref(),
        Some("Invalid Alpaca crypto trades response.")
    );
    assert_eq!(calls, 2);
}

#[test]
fn propagates_output_errors_without_fetching_extra_pages() {
    let mut calls = 0;
    let result = fetch_trades_pages_with(
        3,
        None,
        |_| {
            calls += 1;
            Ok(FIRST.to_vec())
        },
        |_, _| Err("Could not write crypto trades output.".to_owned()),
    );
    assert_eq!(
        result.err().as_deref(),
        Some("Could not write crypto trades output.")
    );
    assert_eq!(calls, 1);
}

#[test]
fn reports_sanitized_auth_rate_limit_provider_and_connection_errors() {
    for (error, expected) in [
        (
            ClientError::UnexpectedStatus(401),
            "Alpaca market data API rejected the credentials (HTTP 401).",
        ),
        (
            ClientError::UnexpectedStatus(429),
            "Alpaca market data API rate limited the request (HTTP 429).",
        ),
        (
            ClientError::UnexpectedStatus(503),
            "Alpaca market data API returned HTTP 503.",
        ),
        (
            ClientError::ConnectionFailed,
            "Could not connect to Alpaca's market data API. Check your network connection.",
        ),
    ] {
        let result = fetch_trades_pages_with(1, None, |_| Err(error.clone()), |_, _| Ok(()));
        assert_eq!(result.err().as_deref(), Some(expected));
        assert!(!expected.contains("example-secret"));
    }
}

#[test]
fn stream_json_and_jsonl_exports_are_private_and_atomic() {
    use crate::output::with_private_output;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    for format in [BarsFormat::Json, BarsFormat::Jsonl] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("trades.json");
        fs::write(&path, "old public data").unwrap();
        #[cfg(unix)]
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        let summary = with_private_output(&path, |writer| {
            write_trades_output(writer, format, 2, None, |token| {
                Ok(if token.is_none() { FIRST } else { SECOND }.to_vec())
            })
        })
        .unwrap();
        assert_eq!(summary.trades, 2);
        assert!(!fs::read(&path).unwrap().is_empty());
        #[cfg(unix)]
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn bad_second_page_rolls_back_partial_jsonl_and_preserves_previous_file() {
    use crate::output::with_private_output;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("trades.jsonl");
    fs::write(&path, "existing data").unwrap();
    let result = with_private_output(&path, |writer| {
        write_trades_output(writer, BarsFormat::Jsonl, 3, None, |token| {
            Ok(if token.is_none() {
                FIRST.to_vec()
            } else {
                br#"{"trades":[]}"#.to_vec()
            })
        })
    });
    assert_eq!(
        result.err().as_deref(),
        Some("Invalid Alpaca crypto trades response.")
    );
    assert_eq!(fs::read(&path).unwrap(), b"existing data");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn http_failure_after_first_page_rolls_back_partial_json_envelope() {
    use crate::output::with_private_output;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("trades.json");
    fs::write(&path, "prior successful data").unwrap();
    let result = with_private_output(&path, |writer| {
        write_trades_output(writer, BarsFormat::Json, 3, None, |token| match token {
            None => Ok(FIRST.to_vec()),
            Some(_) => Err(ClientError::UnexpectedStatus(429)),
        })
    });
    assert_eq!(
        result.err().as_deref(),
        Some("Alpaca market data API rate limited the request (HTTP 429).")
    );
    assert_eq!(fs::read(&path).unwrap(), b"prior successful data");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn validates_cross_zone_ranges_before_credentials_or_api() {
    let mut args = test_args();
    args.start = Some("2026-10-02T00:00:00Z".parse().unwrap());
    args.end = Some("2026-10-02T02:00:00+02:00".parse().unwrap());
    assert_eq!(validate_date_range(&args), Ok(()));
    args.end = Some("2026-10-01".parse().unwrap());
    assert_eq!(
        validate_date_range(&args),
        Err("--start must not be later than --end".into())
    );
}
