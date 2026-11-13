use std::collections::{BTreeMap, HashSet};
use std::io::{self, Write};
use std::path::PathBuf;
use std::str::FromStr;

use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::{Date, OffsetDateTime};

use crate::client::{ClientError, fetch_crypto_bars_page};
use crate::credentials::load_credentials;
use crate::output::save_private_json;

/// Read historical crypto data, not orders or trading activity.
#[derive(Debug, Args, PartialEq, Eq)]
pub struct CryptoBarsArgs {
    /// Required: comma-separated crypto pairs, e.g. BTC/USD,ETH/USD.
    #[arg(long, value_name = "PAIRS")]
    pub symbols: CryptoSymbols,

    /// Location: us (default), us-1, us-2, eu-1 or bs-1.
    #[arg(long, value_enum, default_value_t = CryptoLocation::Us)]
    pub location: CryptoLocation,

    /// Required Alpaca aggregation: e.g. 1Min, 5Min, 1Hour, 1Day.
    #[arg(long, value_name = "TIMEFRAME")]
    pub timeframe: BarsTimeframe,

    /// Inclusive start, YYYY-MM-DD or timezone-qualified RFC3339.
    #[arg(long, value_name = "RFC3339")]
    pub start: Option<BarsInstant>,

    /// Inclusive end, YYYY-MM-DD or timezone-qualified RFC3339.
    #[arg(long, value_name = "RFC3339")]
    pub end: Option<BarsInstant>,

    /// Maximum data points per page across symbols: 1..10000 (default 1000).
    #[arg(long, value_name = "N", default_value_t = 1000, value_parser = parse_page_limit)]
    pub limit: u16,

    /// Sort bars by time: asc (default) or desc.
    #[arg(long, value_enum, default_value_t = BarsSort::Asc)]
    pub sort: BarsSort,

    /// Limit number of pages: 1..20 (default one page).
    #[arg(long, value_name = "N", default_value_t = 1, value_parser = parse_max_pages)]
    pub max_pages: u8,

    /// Resume from a previous response's next_page_token.
    #[arg(long, value_name = "TOKEN")]
    pub page_token: Option<PageToken>,

    /// json (default): summary/complete pages; jsonl: one record per bar.
    #[arg(long, value_enum, default_value_t = BarsFormat::Json)]
    pub format: BarsFormat,

    /// Save JSON page envelopes or JSONL records privately to PATH.
    #[arg(long = "out", value_name = "PATH")]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CryptoSymbols(pub Vec<String>);

impl FromStr for CryptoSymbols {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let symbols: Vec<_> = input.split(',').map(str::trim).collect();
        if (1..=30).contains(&symbols.len())
            && symbols.iter().all(|symbol| {
                let Some((base, quote)) = symbol.split_once('/') else {
                    return false;
                };
                !base.is_empty()
                    && !quote.is_empty()
                    && [base, quote].iter().all(|part| {
                        part.bytes()
                            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
                    })
            })
        {
            Ok(Self(symbols.into_iter().map(str::to_owned).collect()))
        } else {
            Err("Expected 1-30 comma-separated crypto pairs, e.g. BTC/USD,ETH/USD".into())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CryptoLocation {
    #[value(name = "us")]
    Us,
    #[value(name = "us-1")]
    Us1,
    #[value(name = "us-2")]
    Us2,
    #[value(name = "eu-1")]
    Eu1,
    #[value(name = "bs-1")]
    Bs1,
}

impl CryptoLocation {
    pub fn as_path(self) -> &'static str {
        match self {
            Self::Us => "us",
            Self::Us1 => "us-1",
            Self::Us2 => "us-2",
            Self::Eu1 => "eu-1",
            Self::Bs1 => "bs-1",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarsTimeframe(String);

impl BarsTimeframe {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for BarsTimeframe {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let valid = match value {
            "1Day" | "1D" | "1Week" | "1W" => true,
            _ => [
                ("Min", 1..=59, false),
                ("T", 1..=59, false),
                ("Hour", 1..=23, false),
                ("H", 1..=23, false),
                ("Month", 1..=12, true),
                ("M", 1..=12, true),
            ]
            .iter()
            .any(|(suffix, range, months)| {
                value
                    .strip_suffix(suffix)
                    .and_then(|number| number.parse::<u8>().ok().map(|n| (number, n)))
                    .is_some_and(|(raw, n)| {
                        raw == n.to_string()
                            && range.contains(&n)
                            && (!months || matches!(n, 1 | 2 | 3 | 4 | 6 | 12))
                    })
            }),
        };
        if valid {
            Ok(Self(value.to_owned()))
        } else {
            Err("Expected Alpaca timeframe, e.g. 1Min, 5Min, 1Hour, 1Day, 1Week, 3Month".into())
        }
    }
}

/// Retain the exact user-input representation for the HTTP request, while
/// comparing absolute instants to validate cross-timezone start/end bounds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarsInstant {
    raw: String,
    utc: OffsetDateTime,
}

impl BarsInstant {
    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

impl FromStr for BarsInstant {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let utc = Date::parse(input, format_description!("[year]-[month]-[day]"))
            .map(|date| date.midnight().assume_utc())
            .or_else(|_| OffsetDateTime::parse(input, &Rfc3339))
            .map_err(|_| "Expected YYYY-MM-DD or RFC3339 timestamp (with timezone)")?;
        Ok(Self {
            raw: input.to_owned(),
            utc,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum BarsSort {
    Asc,
    Desc,
}

impl BarsSort {
    pub fn as_query(self) -> &'static str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum BarsFormat {
    Json,
    Jsonl,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageToken(String);

impl PageToken {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for PageToken {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if !value.is_empty()
            && value.len() <= 2048
            && value.bytes().all(|byte| byte.is_ascii_graphic())
        {
            Ok(Self(value.to_owned()))
        } else {
            Err("Expected a nonempty printable page token (up to 2048 bytes)".into())
        }
    }
}

fn parse_bounded(value: &str, min: u16, max: u16, name: &str) -> Result<u16, String> {
    if !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && let Ok(number) = value.parse::<u16>()
        && (min..=max).contains(&number)
    {
        return Ok(number);
    }
    Err(format!("{name} must be between {min} and {max}"))
}

fn parse_page_limit(value: &str) -> Result<u16, String> {
    parse_bounded(value, 1, 10000, "Page limit")
}

fn parse_max_pages(value: &str) -> Result<u8, String> {
    parse_bounded(value, 1, 20, "Maximum pages").map(|n| n as u8)
}

pub fn validate_date_range(options: &CryptoBarsArgs) -> Result<(), String> {
    if let (Some(start), Some(end)) = (&options.start, &options.end)
        && start.utc > end.utc
    {
        return Err("--start must not be later than --end".into());
    }
    Ok(())
}

/// Numbers must never pass through f64: serde_json's arbitrary_precision feature
/// preserves the complete upstream decimal text (including long fractions).
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CryptoBar {
    #[serde(rename = "t")]
    pub timestamp: String,
    #[serde(rename = "o")]
    pub open: Number,
    #[serde(rename = "h")]
    pub high: Number,
    #[serde(rename = "l")]
    pub low: Number,
    #[serde(rename = "c")]
    pub close: Number,
    #[serde(rename = "v")]
    pub volume: Number,
    #[serde(rename = "n")]
    pub trade_count: Option<u64>,
    #[serde(rename = "vw")]
    pub vwap: Option<Number>,
}

#[derive(Debug, Deserialize)]
struct ProviderPage {
    bars: BTreeMap<String, Vec<CryptoBar>>,
    next_page_token: Option<String>,
}

#[derive(Debug)]
pub struct CryptoBarsPage {
    pub bars: BTreeMap<String, Vec<CryptoBar>>,
    pub next_page_token: Option<String>,
    // Raw provider page preserves undocumented keys/unknown bar properties.
    pub original: Value,
}

pub fn decode_page(body: &[u8]) -> Result<CryptoBarsPage, serde_json::Error> {
    let original: Value = serde_json::from_slice(body)?;
    let typed: ProviderPage = serde_json::from_value(original.clone())?;
    Ok(CryptoBarsPage {
        bars: typed.bars,
        next_page_token: typed.next_page_token.filter(|token| !token.is_empty()),
        original,
    })
}

pub fn bar_count(pages: &[CryptoBarsPage]) -> usize {
    pages
        .iter()
        .map(|page| page.bars.values().map(Vec::len).sum::<usize>())
        .sum()
}

/// Each exported JSON page is a complete upstream object. A nonnull final
/// next_page_token means the explicitly bounded download was incomplete.
pub fn encode_pages_json(pages: &[CryptoBarsPage]) -> Result<Vec<u8>, serde_json::Error> {
    #[derive(Serialize)]
    struct Export<'a> {
        pages: Vec<&'a Value>,
        next_page_token: Option<&'a str>,
    }
    serde_json::to_vec(&Export {
        pages: pages.iter().map(|page| &page.original).collect(),
        next_page_token: pages
            .last()
            .and_then(|page| page.next_page_token.as_deref()),
    })
}

pub fn encode_bars_jsonl(pages: &[CryptoBarsPage]) -> Result<Vec<u8>, serde_json::Error> {
    #[derive(Serialize)]
    struct Record<'a> {
        symbol: &'a str,
        #[serde(flatten)]
        bar: &'a CryptoBar,
    }
    let mut output = Vec::new();
    for page in pages {
        for (symbol, bars) in &page.bars {
            for bar in bars {
                serde_json::to_writer(
                    &mut output,
                    &Record {
                        symbol: symbol.as_str(),
                        bar,
                    },
                )?;
                output.push(b'\n');
            }
        }
    }
    Ok(output)
}

pub fn render_summary(pages: &[CryptoBarsPage]) -> String {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for page in pages {
        for (symbol, bars) in &page.bars {
            *counts.entry(symbol).or_default() += bars.len();
        }
    }
    let mut result = format!(
        "Medea — Alpaca historical crypto bars\nPages: {}\nBars: {}\n",
        pages.len(),
        bar_count(pages)
    );
    if counts.is_empty() {
        result.push_str("No matching bars.\n");
    } else {
        for (symbol, count) in counts {
            result.push_str(&format!("{symbol}: {count}\n"));
        }
    }
    result.push_str(&format!(
        "More pages available: {}\n",
        if pages
            .last()
            .and_then(|page| page.next_page_token.as_ref())
            .is_some()
        {
            "yes"
        } else {
            "no"
        }
    ));
    result
}

/// Transport injection provides offline pagination tests. Total downloaded
/// history is bounded by validated max_pages (<=20) and limit (<=10000).
/// We do not write partial files if a later page fails decoding or transport.
pub fn fetch_pages_with<F>(
    max_pages: u8,
    initial_token: Option<&str>,
    mut fetch: F,
) -> Result<Vec<CryptoBarsPage>, String>
where
    F: FnMut(Option<&str>) -> Result<Vec<u8>, ClientError>,
{
    if max_pages == 0 || max_pages > 20 {
        return Err("Maximum pages must be between 1 and 20".into());
    }
    let mut pages = Vec::new();
    let mut token = initial_token.map(str::to_owned);
    let mut seen = HashSet::new();
    loop {
        if let Some(current) = &token
            && !seen.insert(current.clone())
        {
            return Err("Alpaca crypto bars returned a repeated page token".into());
        }
        let body = fetch(token.as_deref()).map_err(|err| market_data_error_message(&err))?;
        let page =
            decode_page(&body).map_err(|_| "Invalid Alpaca crypto bars response.".to_owned())?;
        token = page.next_page_token.clone();
        pages.push(page);
        if pages.len() >= usize::from(max_pages) || token.is_none() {
            return Ok(pages);
        }
    }
}

pub fn market_data_error_message(err: &ClientError) -> String {
    match err {
        ClientError::InvalidCredentials => "Invalid Alpaca credentials in environment.".into(),
        ClientError::ConnectionFailed => {
            "Could not connect to Alpaca's market data API. Check your network connection.".into()
        }
        ClientError::UnexpectedStatus(401) => {
            "Alpaca market data API rejected the credentials (HTTP 401).".into()
        }
        ClientError::UnexpectedStatus(429) => {
            "Alpaca market data API rate limited the request (HTTP 429).".into()
        }
        ClientError::UnexpectedStatus(code) => {
            format!("Alpaca market data API returned HTTP {code}.")
        }
        ClientError::OrderNotFound => "Unexpected order-lookup error from market data API.".into(),
    }
}

pub fn run_bars(options: CryptoBarsArgs) -> Result<(), String> {
    validate_date_range(&options)?;
    let credentials = load_credentials().map_err(|err| err.to_string())?;
    let pages = fetch_pages_with(
        options.max_pages,
        options.page_token.as_ref().map(PageToken::as_str),
        |token| fetch_crypto_bars_page(&credentials, &options, token),
    )?;
    let bytes = match options.format {
        BarsFormat::Json => encode_pages_json(&pages),
        BarsFormat::Jsonl => encode_bars_jsonl(&pages),
    }
    .map_err(|_| "Could not encode crypto bars response.".to_owned())?;
    match options.output {
        Some(path) => {
            save_private_json(&path, &bytes)
                .map_err(|err| format!("Could not save crypto bars: {err}"))?;
            println!(
                "Saved {} crypto bars in {} page(s) to {}",
                bar_count(&pages),
                pages.len(),
                path.display()
            );
        }
        None => match options.format {
            BarsFormat::Json => print!("{}", render_summary(&pages)),
            BarsFormat::Jsonl => io::stdout()
                .write_all(&bytes)
                .map_err(|_| "Could not write crypto bars to stdout.".to_owned())?,
        },
    }
    Ok(())
}

#[cfg(test)]
mod tests {
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
        assert!(
            market_data_error_message(&ClientError::ConnectionFailed).contains("market data API")
        );
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
}
