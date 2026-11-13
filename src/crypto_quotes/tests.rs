use super::*;
use serde_json::{Number, Value};
use std::cell::RefCell;
use std::fs;

const FULL: &[u8] = br#"{
 "quotes":{
  "BTC/USD":[{"ap":29059.12345678901234567890123456789012345,"as":0.0000000000000000001234567890123456789,"bp":29058.00000000000000000000001,"bs":0.354400000000000000001,"t":"2026-10-02T00:00:00.123456789Z","future_quote":{"new":true}}],
  "ETH/USD":[{"ap":1817.7,"as":6.137,"bp":1817,"bs":4.76,"t":"2026-10-02T00:00:00.499478272Z"}]
 },
 "next_page_token":"token-two",
 "future_page_metadata":0.123456789012345678901234567890
}"#;
const LAST: &[u8] = br#"{"quotes":{"SOL/USD":[{"ap":300.03,"as":1.5,"bp":300.02,"bs":2.75,"t":"2026-10-02T00:00:01.499478272Z"}]},"next_page_token":null}"#;
const BAD: &[u8] = br#"{"quotes":[]}"#;

fn sample_args() -> CryptoQuotesArgs {
    CryptoQuotesArgs {
        symbols: "BTC/USD,ETH/USD".parse().unwrap(),
        location: CryptoLocation::Us,
        start: None,
        end: None,
        limit: 1000,
        sort: BarsSort::Asc,
        max_pages: 1,
        page_token: None,
        format: BarsFormat::Json,
        output: None,
    }
}

fn precise(raw: &str) -> Number {
    serde_json::from_str(raw).unwrap()
}

#[test]
fn typed_quotes_preserve_all_four_exact_decimal_fields_and_nanosecond_times() {
    let page = decode_page(FULL).unwrap();
    assert_eq!(page.quotes.len(), 2);
    assert_eq!(page.next_page_token.as_deref(), Some("token-two"));
    let btc = &page.quotes["BTC/USD"][0];
    assert_eq!(btc.timestamp, "2026-10-02T00:00:00.123456789Z");
    assert_eq!(
        btc.ask_price,
        precise("29059.12345678901234567890123456789012345")
    );
    assert_eq!(
        btc.ask_size,
        precise("0.0000000000000000001234567890123456789")
    );
    assert_eq!(btc.bid_price, precise("29058.00000000000000000000001"));
    assert_eq!(btc.bid_size, precise("0.354400000000000000001"));
    let eth = &page.quotes["ETH/USD"][0];
    assert_eq!(eth.ask_price, precise("1817.7"));
    assert_eq!(eth.ask_size, precise("6.137"));
    assert_eq!(eth.bid_price, precise("1817"));
    assert_eq!(eth.bid_size, precise("4.76"));
    assert_eq!(eth.timestamp, "2026-10-02T00:00:00.499478272Z");
    assert_eq!(
        page.original["quotes"]["BTC/USD"][0]["future_quote"]["new"],
        true
    );
    assert_eq!(
        page.original["future_page_metadata"],
        Value::Number(precise("0.123456789012345678901234567890"))
    );
}

#[test]
fn empty_quotes_and_optional_tokens_are_not_errors() {
    for raw in [
        r#"{"quotes":{}}"#,
        r#"{"quotes":{},"next_page_token":null}"#,
        r#"{"quotes":{},"next_page_token":""}"#,
        r#"{"quotes":{"BTC/USD":[]},"next_page_token":null}"#,
    ] {
        let page = decode_page(raw.as_bytes()).unwrap();
        assert!(page.quotes.values().all(Vec::is_empty));
        assert_eq!(page.next_page_token, None);
    }
}

#[test]
fn malformed_page_shapes_missing_fields_and_wrong_scalars_are_rejected() {
    for raw in [
        "",
        "[]",
        "{}",
        "{nope",
        r#"{"quotes":null}"#,
        r#"{"quotes":[]}"#,
        r#"{"quotes":{"BTC/USD":[null]}}"#,
        r#"{"quotes":{"BTC/USD":[42]}}"#,
        r#"{"quotes":{"BTC/USD":[{"ap":1,"as":2,"bp":3,"t":"2026-10-02T00:00:00Z"}]}}"#,
        r#"{"quotes":{"BTC/USD":[{"ap":1,"as":2,"bp":3,"bs":4}]}}"#,
        r#"{"quotes":{"BTC/USD":[{"ap":"1","as":2,"bp":3,"bs":4,"t":"2026-10-02T00:00:00Z"}]}}"#,
        r#"{"quotes":{"BTC/USD":[{"ap":null,"as":2,"bp":3,"bs":4,"t":"2026-10-02T00:00:00Z"}]}}"#,
        r#"{"quotes":{"BTC/USD":[{"ap":1,"as":false,"bp":3,"bs":4,"t":"2026-10-02T00:00:00Z"}]}}"#,
        r#"{"quotes":{"BTC/USD":[{"ap":1,"as":2,"bp":3,"bs":4,"t":123}]}}"#,
        r#"{"quotes":{},"next_page_token":123}"#,
    ] {
        assert!(decode_page(raw.as_bytes()).is_err(), "{raw:?}");
    }
}

#[test]
fn validates_timezone_aware_dates_before_io() {
    let mut options = sample_args();
    options.start = Some("2026-10-02T00:00:00.123456789Z".parse().unwrap());
    options.end = Some("2026-10-02T02:00:00.123456789+02:00".parse().unwrap());
    assert_eq!(validate_date_range(&options), Ok(()));
    options.end = Some("2026-10-01".parse().unwrap());
    assert_eq!(
        validate_date_range(&options),
        Err("--start must not be later than --end".into())
    );
}

#[test]
fn exports_two_full_provider_pages_and_preserves_extension_metadata() {
    let mut bytes = Vec::new();
    let summary = write_quotes_output(&mut bytes, BarsFormat::Json, 2, None, |token| {
        Ok(if token.is_none() { FULL } else { LAST }.to_vec())
    })
    .unwrap();
    assert_eq!(summary.pages, 2);
    assert_eq!(summary.quotes, 3);
    assert_eq!(summary.symbols["BTC/USD"], 1);
    assert_eq!(summary.symbols["ETH/USD"], 1);
    assert_eq!(summary.symbols["SOL/USD"], 1);
    assert_eq!(summary.next_page_token, None);
    let result: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(result["pages"][0], decode_page(FULL).unwrap().original);
    assert_eq!(result["pages"][1], decode_page(LAST).unwrap().original);
    assert_eq!(result["next_page_token"], Value::Null);
    assert!(
        String::from_utf8(bytes)
            .unwrap()
            .contains("29059.12345678901234567890123456789012345")
    );
}

#[test]
fn jsonl_writes_exact_one_symbolized_quote_per_line() {
    let mut bytes = Vec::new();
    let summary = write_quotes_output(&mut bytes, BarsFormat::Jsonl, 2, None, |token| {
        Ok(if token.is_none() { FULL } else { LAST }.to_vec())
    })
    .unwrap();
    assert_eq!(summary.quotes, 3);
    let records: Vec<Value> = bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect();
    assert_eq!(records.len(), 3);
    assert_eq!(records[0]["symbol"], "BTC/USD");
    assert_eq!(
        records[0]["ap"],
        Value::Number(precise("29059.12345678901234567890123456789012345"))
    );
    assert_eq!(
        records[0]["bs"],
        Value::Number(precise("0.354400000000000000001"))
    );
    assert_eq!(records[0]["t"], "2026-10-02T00:00:00.123456789Z");
    assert_eq!(records[1]["symbol"], "ETH/USD");
    assert_eq!(records[2]["symbol"], "SOL/USD");
    assert_eq!(records[2]["bp"], Value::Number(precise("300.02")));
}

#[test]
fn empty_quotes_write_no_jsonl_and_summary_is_clear() {
    let mut bytes = Vec::new();
    let summary = write_quotes_output(&mut bytes, BarsFormat::Jsonl, 1, None, |_| {
        Ok(br#"{"quotes":{},"next_page_token":null}"#.to_vec())
    })
    .unwrap();
    assert!(bytes.is_empty());
    assert_eq!(
        render_summary(&summary),
        "Medea — Alpaca historical crypto quotes\nPages: 1\nQuotes: 0\nNo matching quotes.\nMore pages available: no\n"
    );
}

#[test]
fn one_page_default_stops_even_when_token_remains() {
    let calls = RefCell::new(Vec::new());
    let summary = fetch_quotes_pages_with(
        1,
        None,
        |token| {
            calls.borrow_mut().push(token.map(str::to_owned));
            Ok(FULL.to_vec())
        },
        |_, _| Ok(()),
    )
    .unwrap();
    assert_eq!(summary.pages, 1);
    assert_eq!(summary.quotes, 2);
    assert_eq!(summary.next_page_token.as_deref(), Some("token-two"));
    assert_eq!(&*calls.borrow(), &[None]);
    assert!(render_summary(&summary).contains("More pages available: yes"));
}

#[test]
fn fetches_bounded_pages_and_processes_each_once_without_accumulating() {
    let calls = RefCell::new(Vec::new());
    let indexes = RefCell::new(Vec::new());
    let summary = fetch_quotes_pages_with(
        5,
        None,
        |token| {
            calls.borrow_mut().push(token.map(str::to_owned));
            match token {
                None => Ok(FULL.to_vec()),
                Some("token-two") => Ok(LAST.to_vec()),
                _ => Err(ClientError::UnexpectedStatus(400)),
            }
        },
        |index, page| {
            indexes.borrow_mut().push(index);
            assert!(page.quotes.len() <= 2);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(summary.pages, 2);
    assert_eq!(summary.quotes, 3);
    assert_eq!(&*calls.borrow(), &[None, Some("token-two".into())]);
    assert_eq!(&*indexes.borrow(), &[0, 1]);
}

#[test]
fn forwards_opaque_initial_resume_token_unchanged() {
    let calls = RefCell::new(Vec::new());
    let summary = fetch_quotes_pages_with(
        1,
        Some("ABC+/=="),
        |token| {
            calls.borrow_mut().push(token.map(str::to_owned));
            Ok(LAST.to_vec())
        },
        |_, _| Ok(()),
    )
    .unwrap();
    assert_eq!(summary.quotes, 1);
    assert_eq!(&*calls.borrow(), &[Some("ABC+/==".into())]);
}

#[test]
fn repeated_tokens_fail_before_unbounded_requests() {
    let mut calls = 0;
    let error = fetch_quotes_pages_with(
        20,
        None,
        |_| {
            calls += 1;
            Ok(br#"{"quotes":{},"next_page_token":"repeat"}"#.to_vec())
        },
        |_, _| Ok(()),
    )
    .unwrap_err();
    assert_eq!(error, "Alpaca crypto quotes returned a repeated page token");
    assert_eq!(calls, 2);
}

#[test]
fn invalid_page_count_does_not_fetch_and_later_malformed_page_aborts() {
    let mut calls = 0;
    for max_pages in [0, 21] {
        let error = fetch_quotes_pages_with(
            max_pages,
            None,
            |_| {
                calls += 1;
                Ok(FULL.to_vec())
            },
            |_, _| Ok(()),
        )
        .unwrap_err();
        assert_eq!(error, "Maximum pages must be between 1 and 20");
    }
    assert_eq!(calls, 0);
    let error = fetch_quotes_pages_with(
        5,
        None,
        |_| {
            calls += 1;
            Ok(if calls == 1 { FULL } else { BAD }.to_vec())
        },
        |_, _| Ok(()),
    )
    .unwrap_err();
    assert_eq!(error, "Invalid Alpaca crypto quotes response.");
    assert_eq!(calls, 2);
}

#[test]
fn output_error_stops_pagination_without_extra_requests() {
    let mut calls = 0;
    let result = fetch_quotes_pages_with(
        3,
        None,
        |_| {
            calls += 1;
            Ok(FULL.to_vec())
        },
        |_, _| Err("Could not write crypto quotes output.".into()),
    );
    assert_eq!(result.unwrap_err(), "Could not write crypto quotes output.");
    assert_eq!(calls, 1);
}

#[test]
fn auth_rate_limit_network_and_server_failures_remain_sanitized() {
    for (err, expected) in [
        (
            ClientError::UnexpectedStatus(401),
            "Alpaca market data API rejected the credentials (HTTP 401).",
        ),
        (
            ClientError::UnexpectedStatus(403),
            "Alpaca market data API returned HTTP 403.",
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
        let result = fetch_quotes_pages_with(1, None, |_| Err(err.clone()), |_, _| Ok(()));
        assert_eq!(result.unwrap_err(), expected);
    }
}

#[test]
fn private_file_is_atomic_and_owner_only_for_both_formats() {
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    for format in [BarsFormat::Json, BarsFormat::Jsonl] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("quotes.json");
        fs::write(&path, "old public file").unwrap();
        #[cfg(unix)]
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        let summary = with_private_output(&path, |writer| {
            write_quotes_output(writer, format, 2, None, |token| {
                Ok(if token.is_none() { FULL } else { LAST }.to_vec())
            })
        })
        .unwrap();
        assert_eq!(summary.quotes, 3);
        assert!(!fs::read(&path).unwrap().is_empty());
        #[cfg(unix)]
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn later_invalid_page_rolls_back_private_jsonl_file_without_residue() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("quotes.jsonl");
    fs::write(&path, "existing data").unwrap();
    let result = with_private_output(&path, |writer| {
        write_quotes_output(writer, BarsFormat::Jsonl, 3, None, |token| {
            Ok(if token.is_none() { FULL } else { BAD }.to_vec())
        })
    });
    assert_eq!(
        result.unwrap_err(),
        "Invalid Alpaca crypto quotes response."
    );
    assert_eq!(fs::read(&path).unwrap(), b"existing data");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn later_http_429_rolls_back_private_json_envelope() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("quotes.json");
    fs::write(&path, "prior complete data").unwrap();
    let result = with_private_output(&path, |writer| {
        write_quotes_output(writer, BarsFormat::Json, 3, None, |token| match token {
            None => Ok(FULL.to_vec()),
            Some(_) => Err(ClientError::UnexpectedStatus(429)),
        })
    });
    assert_eq!(
        result.unwrap_err(),
        "Alpaca market data API rate limited the request (HTTP 429)."
    );
    assert_eq!(fs::read(&path).unwrap(), b"prior complete data");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}
