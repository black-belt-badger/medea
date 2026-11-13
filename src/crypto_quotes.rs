//! Bounded, read-only historical crypto best bid/ask quote retrieval.
//!
//! Pages are fetched, decoded and consumed one at a time. A later-page
//! failure aborts private atomic output without replacing an existing file.

use std::collections::{BTreeMap, HashSet};
use std::io::{self, Write};
use std::path::PathBuf;

use clap::Args;
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

use crate::client::{ClientError, fetch_crypto_quotes_page};
use crate::credentials::load_credentials;
use crate::crypto_bars::{
    BarsFormat, BarsInstant, BarsSort, CryptoLocation, CryptoSymbols, PageToken,
    market_data_error_message, parse_max_pages, parse_page_limit, validate_instant_range,
};
use crate::output::with_private_output;

#[derive(Debug, Args, PartialEq, Eq)]
pub struct CryptoQuotesArgs {
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

    /// Maximum quotes per page across all symbols: 1..10000 (default 1000).
    #[arg(long, value_name = "N", default_value_t = 1000, value_parser = parse_page_limit)]
    pub limit: u16,

    /// Sort quotes chronologically: asc (default) or desc.
    #[arg(long, value_enum, default_value_t = BarsSort::Asc)]
    pub sort: BarsSort,

    /// Maximum number of pages: 1..20 (default one page).
    #[arg(long, value_name = "N", default_value_t = 1, value_parser = parse_max_pages)]
    pub max_pages: u8,

    /// Resume from the last JSON envelope's next_page_token.
    #[arg(long, value_name = "TOKEN")]
    pub page_token: Option<PageToken>,

    /// json (default): summary or full pages; jsonl: one record per quote.
    #[arg(long, value_enum, default_value_t = BarsFormat::Json)]
    pub format: BarsFormat,

    /// Stream JSON pages or JSONL records privately to PATH.
    #[arg(long = "out", value_name = "PATH")]
    pub output: Option<PathBuf>,
}

pub fn validate_date_range(options: &CryptoQuotesArgs) -> Result<(), String> {
    validate_instant_range(options.start.as_ref(), options.end.as_ref())
}

/// Provider quote: exact JSON decimals rather than f64. The original timestamp
/// text (including nanosecond fractions) is retained without rounding.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CryptoQuote {
    #[serde(rename = "t")]
    pub timestamp: String,
    #[serde(rename = "ap")]
    pub ask_price: Number,
    #[serde(rename = "as")]
    pub ask_size: Number,
    #[serde(rename = "bp")]
    pub bid_price: Number,
    #[serde(rename = "bs")]
    pub bid_size: Number,
}

#[derive(Debug, Deserialize)]
struct ProviderPage {
    quotes: BTreeMap<String, Vec<CryptoQuote>>,
    next_page_token: Option<String>,
}

#[derive(Debug)]
pub struct CryptoQuotesPage {
    pub quotes: BTreeMap<String, Vec<CryptoQuote>>,
    pub next_page_token: Option<String>,
    /// Retain complete provider response, including unknown future properties.
    pub original: Value,
}

pub fn decode_page(bytes: &[u8]) -> Result<CryptoQuotesPage, serde_json::Error> {
    let original: Value = serde_json::from_slice(bytes)?;
    let typed: ProviderPage = serde_json::from_value(original.clone())?;
    Ok(CryptoQuotesPage {
        quotes: typed.quotes,
        next_page_token: typed.next_page_token.filter(|token| !token.is_empty()),
        original,
    })
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct QuotesSummary {
    pub pages: usize,
    pub quotes: usize,
    pub symbols: BTreeMap<String, usize>,
    pub next_page_token: Option<String>,
}

impl QuotesSummary {
    fn add_page(&mut self, page: &CryptoQuotesPage) {
        self.pages += 1;
        self.next_page_token.clone_from(&page.next_page_token);
        for (symbol, quotes) in &page.quotes {
            self.quotes += quotes.len();
            *self.symbols.entry(symbol.clone()).or_default() += quotes.len();
        }
    }
}

pub fn render_summary(summary: &QuotesSummary) -> String {
    let mut output = format!(
        "Medea — Alpaca historical crypto quotes\nPages: {}\nQuotes: {}\n",
        summary.pages, summary.quotes
    );
    if summary.symbols.is_empty() {
        output.push_str("No matching quotes.\n");
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

/// Pages are consumed as they arrive, never accumulated in memory.
pub fn fetch_quotes_pages_with<F, C>(
    max_pages: u8,
    initial_token: Option<&str>,
    mut fetch: F,
    mut consume: C,
) -> Result<QuotesSummary, String>
where
    F: FnMut(Option<&str>) -> Result<Vec<u8>, ClientError>,
    C: FnMut(usize, &CryptoQuotesPage) -> Result<(), String>,
{
    if max_pages == 0 || max_pages > 20 {
        return Err("Maximum pages must be between 1 and 20".into());
    }

    let mut summary = QuotesSummary::default();
    let mut token = initial_token.map(str::to_owned);
    let mut seen = HashSet::new();
    loop {
        if let Some(current) = &token
            && !seen.insert(current.clone())
        {
            return Err("Alpaca crypto quotes returned a repeated page token".into());
        }

        let bytes = fetch(token.as_deref()).map_err(|err| market_data_error_message(&err))?;
        let page =
            decode_page(&bytes).map_err(|_| "Invalid Alpaca crypto quotes response.".to_owned())?;

        consume(summary.pages, &page)?;
        summary.add_page(&page);
        token = page.next_page_token.clone();

        if summary.pages >= usize::from(max_pages) || token.is_none() {
            return Ok(summary);
        }
    }
}

fn output_error(_: impl std::fmt::Display) -> String {
    "Could not write crypto quotes output.".into()
}

#[derive(Serialize)]
struct QuoteRecord<'a> {
    symbol: &'a str,
    #[serde(flatten)]
    quote: &'a CryptoQuote,
}

/// JSON envelopes keep original provider pages; JSONL emits one symbolized
/// quote per line, using arbitrary-precision JSON numeric serialization.
pub fn write_quotes_output<W: Write, F>(
    writer: &mut W,
    format: BarsFormat,
    max_pages: u8,
    initial_token: Option<&str>,
    fetch: F,
) -> Result<QuotesSummary, String>
where
    F: FnMut(Option<&str>) -> Result<Vec<u8>, ClientError>,
{
    if format == BarsFormat::Json {
        writer.write_all(b"{\"pages\":[").map_err(output_error)?;
    }

    let summary = fetch_quotes_pages_with(max_pages, initial_token, fetch, |index, page| {
        match format {
            BarsFormat::Json => {
                if index > 0 {
                    writer.write_all(b",").map_err(output_error)?;
                }
                serde_json::to_writer(&mut *writer, &page.original).map_err(output_error)?;
            }
            BarsFormat::Jsonl => {
                for (symbol, quotes) in &page.quotes {
                    for quote in quotes {
                        serde_json::to_writer(&mut *writer, &QuoteRecord { symbol, quote })
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

pub fn run_quotes(options: CryptoQuotesArgs) -> Result<(), String> {
    validate_date_range(&options)?;
    let credentials = load_credentials().map_err(|err| err.to_string())?;
    let fetch = |token: Option<&str>| fetch_crypto_quotes_page(&credentials, &options, token);
    let initial_token = options.page_token.as_ref().map(PageToken::as_str);

    match &options.output {
        Some(path) => {
            let summary = with_private_output(path, |writer| {
                write_quotes_output(
                    writer,
                    options.format,
                    options.max_pages,
                    initial_token,
                    fetch,
                )
            })?;
            println!(
                "Saved {} crypto quotes in {} page(s) to {}",
                summary.quotes,
                summary.pages,
                path.display()
            );
        }
        None => match options.format {
            BarsFormat::Json => {
                let summary = fetch_quotes_pages_with(
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
                write_quotes_output(
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
