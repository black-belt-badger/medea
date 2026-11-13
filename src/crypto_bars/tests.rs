use super::*;
use std::cell::RefCell;

const RESPONSE: &str = r#"{
      "bars": {
        "BTC/USD": [{
          "t":"2026-10-02T00:00:00.123456789Z",
          "o":29000.123456789012345678,
          "h":29100,
          "l":28900.11,
          "c":29003.120000100,
          "v":0.000000000000123456789,
          "n":4,
          "vw":29001.123456789123456789,
          "future_bar_data":true
        }],
        "ETH/USD": [{
          "t":"2026-10-02T00:00:00Z",
          "o":2200,"h":2205,"l":2199,"c":2201,"v":3.5,
          "n":null,"vw":null
        }]
      },
      "next_page_token":"token-1",
      "future_page_metadata":42
    }"#;
const FIRST: &[u8] = br#"{"bars":{"BTC/USD":[{"t":"2026-10-01T00:00:00.123456789Z","o":1,"h":2,"l":1,"c":2,"v":0.001,"n":1,"vw":1.5}]},"next_page_token":"token-2"}"#;
const SECOND: &[u8] = br#"{"bars":{"ETH/USD":[{"t":"2026-10-01T01:00:00Z","o":3,"h":4,"l":3,"c":4,"v":0.002,"n":2,"vw":3.5}]},"next_page_token":null}"#;

fn parsed(value: &str) -> CryptoBarsPage {
    decode_page(value.as_bytes()).unwrap()
}

#[test]
fn parses_crypto_pairs_and_rejects_malformed_or_injected_symbols() {
    assert_eq!(
        "BTC/USD, ETH/USD,SOL/USD"
            .parse::<CryptoSymbols>()
            .unwrap()
            .0,
        vec!["BTC/USD", "ETH/USD", "SOL/USD"]
    );
    assert_eq!(
        "DOGE/USD".parse::<CryptoSymbols>().unwrap().0,
        vec!["DOGE/USD"]
    );
    for invalid in [
        "",
        "BTCUSD",
        "BTC/USD,",
        ",BTC/USD",
        "BTC/USD,,ETH/USD",
        "btc/USD",
        "BTC/US D",
        "BTC/USD&limit=10000",
        "BTC/USD/USDT",
        "../BTC/USD",
        "BTC/USD?sort=desc",
    ] {
        assert!(invalid.parse::<CryptoSymbols>().is_err(), "{invalid:?}");
    }
    let excessive = std::iter::repeat_n("BTC/USD", 31)
        .collect::<Vec<_>>()
        .join(",");
    assert!(excessive.parse::<CryptoSymbols>().is_err());
}

#[test]
fn supports_documented_timeframes_and_rejects_noncanonical_values() {
    for valid in [
        "1Min", "5Min", "59Min", "1T", "59T", "1Hour", "23Hour", "12H", "1Day", "1D", "1Week",
        "1W", "1Month", "2Month", "3Month", "4Month", "6Month", "12Month", "12M",
    ] {
        assert_eq!(valid.parse::<BarsTimeframe>().unwrap().as_str(), valid);
    }
    for invalid in [
        "",
        "0Min",
        "60Min",
        "00Min",
        "1min",
        "24Hour",
        "2Day",
        "2Week",
        "5Month",
        "13M",
        "1Second",
        "1T?limit=10000",
        "1.5Hour",
    ] {
        assert!(invalid.parse::<BarsTimeframe>().is_err(), "{invalid:?}");
    }
}

#[test]
fn validates_dates_and_preserves_precise_input_and_cross_zone_comparison() {
    for valid in [
        "2026-10-02",
        "2026-10-02T00:00:00Z",
        "2026-10-02T00:00:00.123456789Z",
        "2026-10-02T02:00:00+02:00",
        "2026-10-02T02:00:00.987654321+02:00",
    ] {
        assert_eq!(valid.parse::<BarsInstant>().unwrap().as_str(), valid);
    }
    for invalid in [
        "",
        "today",
        "2026-02-30",
        "2026-13-01",
        "2026-10-02T00:00:00",
        "2026-10-02T00:00:00Z&limit=10000",
    ] {
        assert!(invalid.parse::<BarsInstant>().is_err(), "{invalid:?}");
    }
    let mut args = sample_args();
    args.start = Some("2026-10-02T00:00:00Z".parse().unwrap());
    args.end = Some("2026-10-02T02:00:00+02:00".parse().unwrap());
    assert!(validate_date_range(&args).is_ok());
    args.end = Some("2026-10-01".parse().unwrap());
    assert_eq!(
        validate_date_range(&args),
        Err("--start must not be later than --end".into())
    );
    args.start = None;
    assert!(validate_date_range(&args).is_ok());
}

#[test]
fn validates_page_boundaries_and_opaque_tokens() {
    for n in ["1", "1000", "10000"] {
        assert!(parse_page_limit(n).is_ok());
    }
    for n in ["", "0", "10001", "-1", "1.5", "99999999999999999999999"] {
        assert!(parse_page_limit(n).is_err(), "{n:?}");
    }
    for n in ["1", "2", "20"] {
        assert!(parse_max_pages(n).is_ok());
    }
    for n in ["", "0", "21", "-1", "999999999999999999"] {
        assert!(parse_max_pages(n).is_err(), "{n:?}");
    }
    assert_eq!(
        "abc/123+==".parse::<PageToken>().unwrap().as_str(),
        "abc/123+=="
    );
    for invalid in ["", "has space", "has\nnewline"] {
        assert!(invalid.parse::<PageToken>().is_err());
    }
    assert!("a".repeat(2049).parse::<PageToken>().is_err());
}

#[test]
fn decodes_symbolized_bars_with_exact_decimal_values_and_nanosecond_time() {
    let page = parsed(RESPONSE);
    assert_eq!(bar_count(&[parsed(RESPONSE)]), 2);
    assert_eq!(page.next_page_token.as_deref(), Some("token-1"));
    assert_eq!(page.bars.len(), 2);
    let btc = &page.bars["BTC/USD"][0];
    assert_eq!(btc.timestamp, "2026-10-02T00:00:00.123456789Z");
    assert_eq!(btc.open.to_string(), "29000.123456789012345678");
    assert_eq!(btc.volume.to_string(), "1.23456789e-13");
    assert_eq!(btc.close.to_string(), "29003.120000100");
    assert_eq!(
        btc.vwap.as_ref().unwrap().to_string(),
        "29001.123456789123456789"
    );
    assert_eq!(btc.trade_count, Some(4));
    let eth = &page.bars["ETH/USD"][0];
    assert_eq!(eth.trade_count, None);
    assert_eq!(eth.vwap, None);
}

#[test]
fn tolerates_optional_null_and_missing_fields() {
    let page = parsed(
        r#"{"bars":{"SOL/USD":[{"t":"2026-10-02T00:00:00Z","o":1,"h":1,"l":1,"c":1,"v":0}]},"next_page_token":null}"#,
    );
    assert!(page.next_page_token.is_none());
    assert_eq!(page.bars["SOL/USD"][0].trade_count, None);
    assert_eq!(page.bars["SOL/USD"][0].vwap, None);
    assert_eq!(
        parsed(r#"{"bars":{},"next_page_token":""}"#).next_page_token,
        None
    );
}

#[test]
fn rejects_malformed_empty_wrong_shape_and_nonnumeric_fields() {
    for body in [
        "",
        "[]",
        "{}",
        "{broken",
        r#"{"bars":null}"#,
        r#"{"bars":[1,2,3]}"#,
        r#"{"bars":{"BTC/USD":[{"o":1}]}}"#,
        r#"{"bars":{"BTC/USD":[{"t":"2026-10-01T00:00:00Z","o":"one","h":1,"l":1,"c":1,"v":2}]}}"#,
    ] {
        assert!(decode_page(body.as_bytes()).is_err(), "{body:?}");
    }
}

#[test]
fn exports_all_original_pages_including_unknown_keys_and_precision() {
    let pages = vec![parsed(RESPONSE), parsed(RESPONSE)];
    let output = encode_pages_json(&pages).unwrap();
    let exported: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(exported["pages"][0], pages[0].original);
    assert_eq!(exported["pages"][1], pages[1].original);
    assert_eq!(exported["pages"][0]["future_page_metadata"], 42);
    assert_eq!(
        exported["pages"][0]["bars"]["BTC/USD"][0]["future_bar_data"],
        true
    );
    assert_eq!(exported["next_page_token"], "token-1");
    let literal = String::from_utf8(output).unwrap();
    assert!(literal.contains("29000.123456789012345678"));
}

#[test]
fn jsonl_emits_one_record_per_bar_with_exact_numbers_and_symbol() {
    let output = encode_bars_jsonl(&[parsed(RESPONSE)]).unwrap();
    let lines: Vec<_> = output
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .collect();
    assert_eq!(lines.len(), 2);
    let records: Vec<Value> = lines
        .iter()
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect();
    assert_eq!(records[0]["symbol"], "BTC/USD");
    assert_eq!(records[1]["symbol"], "ETH/USD");
    assert_eq!(records[0]["t"], "2026-10-02T00:00:00.123456789Z");
    assert!(
        String::from_utf8(output)
            .unwrap()
            .contains("29000.123456789012345678")
    );
    assert_eq!(records[0]["n"], 4);
    assert_eq!(records[1]["n"], Value::Null);
    assert_eq!(records[1]["vw"], Value::Null);
}

#[test]
fn empty_page_is_valid_and_summary_is_clear() {
    let page = parsed(r#"{"bars":{}}"#);
    assert_eq!(bar_count(&[parsed(r#"{"bars":{}}"#)]), 0);
    assert!(encode_bars_jsonl(&[page]).unwrap().is_empty());
    assert_eq!(
        render_summary(&[parsed(r#"{"bars":{}}"#)]),
        "Medea — Alpaca historical crypto bars\nPages: 1\nBars: 0\nNo matching bars.\nMore pages available: no\n"
    );
}

#[test]
fn one_page_default_does_not_follow_provider_token() {
    let calls = RefCell::new(Vec::new());
    let pages = fetch_pages_with(1, None, |token| {
        calls.borrow_mut().push(token.map(str::to_owned));
        Ok(FIRST.to_vec())
    })
    .unwrap();
    assert_eq!(bar_count(&pages), 1);
    assert_eq!(&*calls.borrow(), &[None]);
    assert!(render_summary(&pages).contains("More pages available: yes"));
}

#[test]
fn follows_tokens_until_end_or_explicit_page_bound() {
    let calls = RefCell::new(Vec::new());
    let pages = fetch_pages_with(5, None, |token| {
        calls.borrow_mut().push(token.map(str::to_owned));
        match token {
            None => Ok(FIRST.to_vec()),
            Some("token-2") => Ok(SECOND.to_vec()),
            _ => Err(ClientError::UnexpectedStatus(400)),
        }
    })
    .unwrap();
    assert_eq!(bar_count(&pages), 2);
    assert_eq!(&*calls.borrow(), &[None, Some("token-2".into())]);
    assert!(render_summary(&pages).contains("BTC/USD: 1"));
    assert!(render_summary(&pages).contains("ETH/USD: 1"));
    assert!(render_summary(&pages).contains("More pages available: no"));
}

#[test]
fn supplied_initial_page_token_is_used_verbatim() {
    let calls = RefCell::new(Vec::new());
    let pages = fetch_pages_with(1, Some("abc+/=="), |token| {
        calls.borrow_mut().push(token.map(str::to_owned));
        Ok(SECOND.to_vec())
    })
    .unwrap();
    assert_eq!(bar_count(&pages), 1);
    assert_eq!(&*calls.borrow(), &[Some("abc+/==".into())]);
}

#[test]
fn rejects_repeated_tokens_without_unbounded_calls() {
    let mut count = 0;
    let result = fetch_pages_with(20, None, |_| {
        count += 1;
        Ok(br#"{"bars":{},"next_page_token":"repeat"}"#.to_vec())
    });
    assert_eq!(
        result.err().as_deref(),
        Some("Alpaca crypto bars returned a repeated page token")
    );
    assert_eq!(count, 2);
}

#[test]
fn rejects_out_of_range_max_pages_before_networking() {
    let mut calls = 0;
    for max_pages in [0, 21] {
        assert!(
            fetch_pages_with(max_pages, None, |_| {
                calls += 1;
                Ok(FIRST.to_vec())
            })
            .is_err()
        );
    }
    assert_eq!(calls, 0);
}

#[test]
fn malformed_later_page_aborts_without_returning_partial_data() {
    let mut calls = 0;
    let result = fetch_pages_with(3, None, |_| {
        calls += 1;
        Ok(if calls == 1 {
            FIRST.to_vec()
        } else {
            br#"{"bars":[]}"#.to_vec()
        })
    });
    assert_eq!(
        result.err().as_deref(),
        Some("Invalid Alpaca crypto bars response.")
    );
    assert_eq!(calls, 2);
}

#[test]
fn market_data_errors_are_sanitized_and_distinguish_rate_limits() {
    assert_eq!(
        market_data_error_message(&ClientError::UnexpectedStatus(401)),
        "Alpaca market data API rejected the credentials (HTTP 401)."
    );
    assert_eq!(
        market_data_error_message(&ClientError::UnexpectedStatus(429)),
        "Alpaca market data API rate limited the request (HTTP 429)."
    );
    assert_eq!(
        market_data_error_message(&ClientError::UnexpectedStatus(503)),
        "Alpaca market data API returned HTTP 503."
    );
    assert!(market_data_error_message(&ClientError::ConnectionFailed).contains("market data API"));
    assert!(!market_data_error_message(&ClientError::ConnectionFailed).contains("secret"));
}

fn sample_args() -> CryptoBarsArgs {
    CryptoBarsArgs {
        symbols: "BTC/USD,ETH/USD".parse().unwrap(),
        location: CryptoLocation::Us,
        timeframe: "1Hour".parse().unwrap(),
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
