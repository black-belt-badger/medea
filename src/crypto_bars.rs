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

pub(crate) fn parse_page_limit(value: &str) -> Result<u16, String> {
    parse_bounded(value, 1, 10000, "Page limit")
}

pub(crate) fn parse_max_pages(value: &str) -> Result<u8, String> {
    parse_bounded(value, 1, 20, "Maximum pages").map(|n| n as u8)
}

pub fn validate_date_range(options: &CryptoBarsArgs) -> Result<(), String> {
    validate_instant_range(options.start.as_ref(), options.end.as_ref())
}

pub(crate) fn validate_instant_range(
    start: Option<&BarsInstant>,
    end: Option<&BarsInstant>,
) -> Result<(), String> {
    if let (Some(start), Some(end)) = (start, end)
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
        ClientError::ConfirmationRequired | ClientError::OrderNotCancelable => {
            "Unexpected paper-order cancellation error from market data API.".into()
        }
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
mod tests;
