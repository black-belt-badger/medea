//! Bounded, read-only historical crypto trade retrieval.
//!
//! One API page is decoded, consumed and dropped before the next request.
//! Atomic private exports leave existing files untouched on later-page errors.

use std::collections::{BTreeMap, HashSet};
use std::io::{self, Write};
use std::path::PathBuf;

use clap::Args;
use serde::{Deserialize, Deserializer, Serialize, de};
use serde_json::{Number, Value};

use crate::client::{ClientError, fetch_crypto_trades_page};
use crate::credentials::load_credentials;
use crate::crypto_bars::{
    BarsFormat, BarsInstant, BarsSort, CryptoLocation, CryptoSymbols, PageToken,
    market_data_error_message, parse_max_pages, parse_page_limit, validate_instant_range,
};
use crate::output::with_private_output;

#[derive(Debug, Args, PartialEq, Eq)]
pub struct CryptoTradesArgs {
    /// Required: comma-separated crypto pairs, e.g. BTC/USD,ETH/USD.
    #[arg(long, value_name = "PAIRS")]
    pub symbols: CryptoSymbols,

    /// Location: us (default), us-1, us-2, eu-1 or bs-1.
    #[arg(long, value_enum, default_value_t = CryptoLocation::Us)]
    pub location: CryptoLocation,

    /// Inclusive start, YYYY-MM-DD or timezone-qualified RFC3339.
    #[arg(long, value_name = "RFC3339")]
    pub start: Option<BarsInstant>,

    /// Inclusive end, YYYY-MM-DD or timezone-qualified RFC3339.
    #[arg(long, value_name = "RFC3339")]
    pub end: Option<BarsInstant>,

    /// Maximum trades per page across symbols: 1..10000 (default 1000).
    #[arg(long, value_name = "N", default_value_t = 1000, value_parser = parse_page_limit)]
    pub limit: u16,

    /// Sort trades chronologically: asc (default) or desc.
    #[arg(long, value_enum, default_value_t = BarsSort::Asc)]
    pub sort: BarsSort,

    /// Maximum number of pages: 1..20 (default one page).
    #[arg(long, value_name = "N", default_value_t = 1, value_parser = parse_max_pages)]
    pub max_pages: u8,

    /// Resume from the last JSON envelope's next_page_token.
    #[arg(long, value_name = "TOKEN")]
    pub page_token: Option<PageToken>,

    /// json (default): summary/complete pages; jsonl: one record per trade.
    #[arg(long, value_enum, default_value_t = BarsFormat::Json)]
    pub format: BarsFormat,

    /// Stream JSON pages or JSONL records privately to PATH.
    #[arg(long = "out", value_name = "PATH")]
    pub output: Option<PathBuf>,
}

pub fn validate_date_range(options: &CryptoTradesArgs) -> Result<(), String> {
    validate_instant_range(options.start.as_ref(), options.end.as_ref())
}

/// IDs remain arbitrary-size exact integers, not i64/u64/f64; prices and
/// sizes use arbitrary_precision serde_json::Number. Timestamps are verbatim.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CryptoTrade {
    #[serde(rename = "i", deserialize_with = "deserialize_trade_id")]
    pub id: Number,
    #[serde(rename = "p")]
    pub price: Number,
    #[serde(rename = "s")]
    pub size: Number,
    #[serde(rename = "t")]
    pub timestamp: String,
    #[serde(rename = "tks")]
    pub taker_side: Option<String>,
}

fn deserialize_trade_id<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Number, D::Error> {
    let id = Number::deserialize(deserializer)?;
    if id.to_string().bytes().all(|b| b.is_ascii_digit()) {
        Ok(id)
    } else {
        Err(de::Error::custom("expected an unsigned integer trade ID"))
    }
}

#[derive(Debug, Deserialize)]
struct ProviderPage {
    trades: BTreeMap<String, Vec<CryptoTrade>>,
    next_page_token: Option<String>,
}

#[derive(Debug)]
pub struct CryptoTradesPage {
    pub trades: BTreeMap<String, Vec<CryptoTrade>>,
    pub next_page_token: Option<String>,
    pub original: Value,
}

pub fn decode_page(bytes: &[u8]) -> Result<CryptoTradesPage, serde_json::Error> {
    let original: Value = serde_json::from_slice(bytes)?;
    let typed: ProviderPage = serde_json::from_value(original.clone())?;
    Ok(CryptoTradesPage {
        trades: typed.trades,
        next_page_token: typed.next_page_token.filter(|token| !token.is_empty()),
        original,
    })
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct TradesSummary {
    pub pages: usize,
    pub trades: usize,
    pub symbols: BTreeMap<String, usize>,
    pub next_page_token: Option<String>,
}

impl TradesSummary {
    fn add_page(&mut self, page: &CryptoTradesPage) {
        self.pages += 1;
        self.next_page_token.clone_from(&page.next_page_token);
        for (symbol, trades) in &page.trades {
            self.trades += trades.len();
            *self.symbols.entry(symbol.clone()).or_default() += trades.len();
        }
    }
}

pub fn render_summary(summary: &TradesSummary) -> String {
    let mut output = format!(
        "Medea — Alpaca historical crypto trades\nPages: {}\nTrades: {}\n",
        summary.pages, summary.trades
    );
    if summary.symbols.is_empty() {
        output.push_str("No matching trades.\n");
    } else {
        for (symbol, count) in &summary.symbols {
            output.push_str(&format!("{symbol}: {count}\n"));
        }
    }
    output.push_str(&format!(
        "More pages available: {}\n",
        if summary.next_page_token.is_some() {
            "yes"
        } else {
            "no"
        }
    ));
    output
}

/// The callback sees just one page at a time. In particular, file writers
/// never need to buffer previous pages or accumulate a large Vec of trades.
pub fn fetch_trades_pages_with<F, C>(
    max_pages: u8,
    initial_token: Option<&str>,
    mut fetch: F,
    mut consume: C,
) -> Result<TradesSummary, String>
where
    F: FnMut(Option<&str>) -> Result<Vec<u8>, ClientError>,
    C: FnMut(usize, &CryptoTradesPage) -> Result<(), String>,
{
    if max_pages == 0 || max_pages > 20 {
        return Err("Maximum pages must be between 1 and 20".into());
    }

    let mut summary = TradesSummary::default();
    let mut token = initial_token.map(str::to_owned);
    let mut seen = HashSet::new();
    loop {
        if let Some(current) = &token
            && !seen.insert(current.clone())
        {
            return Err("Alpaca crypto trades returned a repeated page token".into());
        }
        let bytes = fetch(token.as_deref()).map_err(|err| market_data_error_message(&err))?;
        let page =
            decode_page(&bytes).map_err(|_| "Invalid Alpaca crypto trades response.".to_owned())?;

        consume(summary.pages, &page)?;
        summary.add_page(&page);
        token = page.next_page_token.clone();

        if summary.pages >= usize::from(max_pages) || token.is_none() {
            return Ok(summary);
        }
    }
}

fn output_error(_: impl std::fmt::Display) -> String {
    "Could not write crypto trades output.".into()
}

#[derive(Serialize)]
struct TradeRecord<'a> {
    symbol: &'a str,
    #[serde(flatten)]
    trade: &'a CryptoTrade,
}

/// Output matches Kirke: JSON contains original provider pages and last token;
/// JSONL is one typed, symbolized trade per line.
pub fn write_trades_output<W: Write, F>(
    writer: &mut W,
    format: BarsFormat,
    max_pages: u8,
    initial_token: Option<&str>,
    fetch: F,
) -> Result<TradesSummary, String>
where
    F: FnMut(Option<&str>) -> Result<Vec<u8>, ClientError>,
{
    if format == BarsFormat::Json {
        writer.write_all(b"{\"pages\":[").map_err(output_error)?;
    }

    let summary = fetch_trades_pages_with(max_pages, initial_token, fetch, |index, page| {
        match format {
            BarsFormat::Json => {
                if index > 0 {
                    writer.write_all(b",").map_err(output_error)?;
                }
                serde_json::to_writer(&mut *writer, &page.original).map_err(output_error)?;
            }
            BarsFormat::Jsonl => {
                for (symbol, trades) in &page.trades {
                    for trade in trades {
                        serde_json::to_writer(&mut *writer, &TradeRecord { symbol, trade })
                            .map_err(output_error)?;
                        writer.write_all(b"\n").map_err(output_error)?;
                    }
                }
            }
        }
        Ok(())
    })?;

    if format == BarsFormat::Json {
        writer
            .write_all(b"],\"next_page_token\":")
            .map_err(output_error)?;
        serde_json::to_writer(&mut *writer, &summary.next_page_token).map_err(output_error)?;
        writer.write_all(b"}").map_err(output_error)?;
    }
    Ok(summary)
}

pub fn run_trades(options: CryptoTradesArgs) -> Result<(), String> {
    validate_date_range(&options)?;
    let credentials = load_credentials().map_err(|err| err.to_string())?;
    let fetch = |token: Option<&str>| fetch_crypto_trades_page(&credentials, &options, token);
    let initial_token = options.page_token.as_ref().map(PageToken::as_str);

    match &options.output {
        Some(path) => {
            let summary = with_private_output(path, |writer| {
                write_trades_output(
                    writer,
                    options.format,
                    options.max_pages,
                    initial_token,
                    fetch,
                )
            })?;
            println!(
                "Saved {} crypto trades in {} page(s) to {}",
                summary.trades,
                summary.pages,
                path.display()
            );
        }
        None => match options.format {
            BarsFormat::Json => {
                let summary = fetch_trades_pages_with(
                    options.max_pages,
                    initial_token,
                    fetch,
                    |_, _| Ok(()),
                )?;
                print!("{}", render_summary(&summary));
            }
            BarsFormat::Jsonl => {
                let stdout = io::stdout();
                let mut writer = stdout.lock();
                write_trades_output(
                    &mut writer,
                    BarsFormat::Jsonl,
                    options.max_pages,
                    initial_token,
                    fetch,
                )?;
            }
        },
    }
    Ok(())
}

#[cfg(test)]
mod tests;
