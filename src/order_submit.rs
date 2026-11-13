//! Dry-run-first, validated crypto PAPER order submission.
//!
//! No account writes occur unless an operator supplies --confirm. All input
//! validation precedes credential loading and any network operation.

use std::path::PathBuf;
use std::str::FromStr;

use clap::{Args, ValueEnum};
use serde::Serialize;

use crate::client::{ClientError, fetch_submit_order};
use crate::credentials::load_credentials;
use crate::orders::{Order, decode_order, render_order_summary};
use crate::output::save_private_json;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitSymbol(String);

impl SubmitSymbol {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for SubmitSymbol {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let valid = input.split_once('/').is_some_and(|(base, quote)| {
            !base.is_empty()
                && !quote.is_empty()
                && base
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
                && quote
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
        });
        if valid {
            Ok(Self(input.to_owned()))
        } else {
            Err("Expected one crypto pair, e.g. BTC/USD (not a comma-separated list)".into())
        }
    }
}

/// Exact positive decimal input, never parsed into f32 or f64.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitAmount(String);

impl SubmitAmount {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn fractional_digits(&self) -> usize {
        self.0
            .split_once('.')
            .map_or(0, |(_, fraction)| fraction.len())
    }
}

impl FromStr for SubmitAmount {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let (whole, fraction) = match input.split_once('.') {
            Some((whole, fraction)) => (whole, Some(fraction)),
            None => (input, None),
        };
        let valid = !input.is_empty()
            && input.len() <= 64
            && !whole.is_empty()
            && whole.bytes().all(|byte| byte.is_ascii_digit())
            && fraction.is_none_or(|value| {
                !value.is_empty()
                    && value.len() <= 9
                    && value.bytes().all(|byte| byte.is_ascii_digit())
            })
            && input.bytes().any(|byte| (b'1'..=b'9').contains(&byte));
        if valid {
            Ok(Self(input.to_owned()))
        } else {
            Err("Expected a positive decimal (up to 9 fractional digits, no exponent)".into())
        }
    }
}

/// Caller-supplied and stable across previews and uncertain responses.
/// This ID does NOT guarantee exactly-once execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientOrderId(String);

impl ClientOrderId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for ClientOrderId {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let valid = !input.is_empty()
            && input.len() <= 128
            && input.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':')
            });
        if valid {
            Ok(Self(input.to_owned()))
        } else {
            Err("Expected a nonempty client order ID (max 128 ASCII letters, digits, '.', '-', '_' or ':')".into())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SubmitSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SubmitKind {
    Market,
    Limit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SubmitTimeInForce {
    #[default]
    Gtc,
    Ioc,
}

#[derive(Debug, Args, Clone, PartialEq, Eq)]
pub struct SubmitOrderArgs {
    /// Exactly one uppercase crypto pair (e.g., BTC/USD).
    #[arg(long, value_name = "PAIR")]
    pub symbol: SubmitSymbol,
    /// buy or sell.
    #[arg(long, value_enum)]
    pub side: SubmitSide,
    /// market or limit; advanced order classes are not supported here.
    #[arg(long = "type", value_enum)]
    pub kind: SubmitKind,
    /// gtc (default) or ioc.
    #[arg(long = "time-in-force", value_enum, default_value_t = SubmitTimeInForce::Gtc)]
    pub time_in_force: SubmitTimeInForce,
    /// Positive exact quantity; exclusive with --notional.
    #[arg(long, value_name = "DECIMAL")]
    pub qty: Option<SubmitAmount>,
    /// Positive exact notional; market orders only; exclusive with --qty.
    #[arg(long, value_name = "DECIMAL")]
    pub notional: Option<SubmitAmount>,
    /// Required for limit orders, forbidden for market.
    #[arg(long = "limit-price", value_name = "DECIMAL")]
    pub limit_price: Option<SubmitAmount>,
    /// Required stable identifier (no auto-generated order IDs).
    #[arg(long = "client-order-id", value_name = "ID")]
    pub client_order_id: ClientOrderId,
    /// Actually send ONE order to Alpaca's PAPER API; default is offline dry run.
    #[arg(long)]
    pub confirm: bool,
    /// Save verified original JSON response privately (requires --confirm).
    #[arg(long = "out", value_name = "PATH")]
    pub output: Option<PathBuf>,
}

/// Only a successfully validated request can reach the POST builder.
/// Neither credentials, confirmation status nor output file are serialized.
#[derive(Debug, Serialize)]
pub struct PreparedOrder<'a> {
    symbol: &'a str,
    side: SubmitSide,
    #[serde(rename = "type")]
    kind: SubmitKind,
    time_in_force: SubmitTimeInForce,
    client_order_id: &'a str,
    #[serde(skip)]
    confirmed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    qty: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    notional: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit_price: Option<&'a str>,
}

impl PreparedOrder<'_> {
    pub fn client_order_id(&self) -> &str {
        self.client_order_id
    }

    /// A second confirmation gate lives in the POST builder itself.
    pub fn confirmed(&self) -> bool {
        self.confirmed
    }

    pub fn body(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self).map_err(|_| "Could not serialize paper-order request.".into())
    }
}

impl SubmitOrderArgs {
    /// Must be called even for dry runs, prior to credential or network access.
    pub fn prepare(&self) -> Result<PreparedOrder<'_>, String> {
        match (&self.qty, &self.notional) {
            (None, None) => return Err("Exactly one of --qty or --notional is required".into()),
            (Some(_), Some(_)) => return Err("--qty and --notional cannot be combined".into()),
            _ => {}
        }
        match self.kind {
            SubmitKind::Market if self.limit_price.is_some() => {
                return Err("--limit-price is only allowed for limit orders".into());
            }
            SubmitKind::Limit if self.limit_price.is_none() => {
                return Err("Limit orders require --limit-price".into());
            }
            SubmitKind::Limit if self.notional.is_some() => {
                return Err(
                    "Limit orders require --qty; --notional is supported only for market orders"
                        .into(),
                );
            }
            _ => {}
        }
        if self.symbol.as_str().ends_with("/USD")
            && self
                .notional
                .as_ref()
                .is_some_and(|amount| amount.fractional_digits() > 2)
        {
            return Err("USD-quoted crypto notional supports at most 2 decimal places".into());
        }
        if !self.confirm && self.output.is_some() {
            return Err("--out requires --confirm; dry runs do not write response files".into());
        }

        Ok(PreparedOrder {
            symbol: self.symbol.as_str(),
            side: self.side,
            kind: self.kind,
            time_in_force: self.time_in_force,
            client_order_id: self.client_order_id.as_str(),
            confirmed: self.confirm,
            qty: self.qty.as_ref().map(SubmitAmount::as_str),
            notional: self.notional.as_ref().map(SubmitAmount::as_str),
            limit_price: self.limit_price.as_ref().map(SubmitAmount::as_str),
        })
    }
}

pub fn dry_run_message(prepared: &PreparedOrder<'_>) -> Result<String, String> {
    let body = String::from_utf8(prepared.body()?)
        .map_err(|_| "Could not serialize paper-order request.".to_owned())?;
    Ok(format!(
        "DRY RUN — no credentials loaded; no order submitted.\n\
         Review the intended paper order below; use --confirm to submit.\n\
         A reused client_order_id does not guarantee exactly-once delivery.\n\
         {body}\n"
    ))
}

pub fn submission_error_message(error: &ClientError) -> String {
    match error {
        ClientError::ConnectionFailed => "Paper order submission result unknown (network failure). \
            Check existing paper orders by client_order_id before retrying.".into(),
        ClientError::UnexpectedStatus(401) => {
            "Alpaca paper API rejected credentials (HTTP 401). No response body exposed.".into()
        }
        ClientError::UnexpectedStatus(403) => {
            "Alpaca paper order rejected (HTTP 403; check permissions or buying power).".into()
        }
        ClientError::UnexpectedStatus(422) => {
            "Alpaca paper order rejected (HTTP 422; check order parameters and asset rules).".into()
        }
        ClientError::UnexpectedStatus(429) => {
            "Alpaca paper API rate-limited submission (HTTP 429). Check paper orders before retrying.".into()
        }
        ClientError::UnexpectedStatus(code) if *code >= 500 => format!(
            "Alpaca paper API returned HTTP {code} during submission. Check paper orders before retrying."
        ),
        ClientError::UnexpectedStatus(code) => {
            format!("Alpaca paper order submission returned HTTP {code}.")
        }
        ClientError::InvalidCredentials => "Invalid Alpaca credentials in environment.".into(),
        ClientError::ConfirmationRequired => {
            "Paper order submission requires explicit --confirm.".into()
        }
        ClientError::OrderNotCancelable => {
            "Alpaca returned an unexpected cancellation error during submission.".into()
        }
        ClientError::OrderNotFound => "Alpaca paper order not found (HTTP 404).".into(),
    }
}

/// A POST response may indicate acceptance, NOT execution or a fill.
/// Validate the matching client ID before any display or private export.
pub fn decode_submit_result(
    prepared: &PreparedOrder<'_>,
    response: Result<Vec<u8>, ClientError>,
) -> Result<(Order, Vec<u8>), String> {
    let raw = response.map_err(|error| submission_error_message(&error))?;
    let order = decode_order(&raw).map_err(|_| {
        "Invalid paper-order submission response. Check paper orders before retrying.".to_owned()
    })?;
    if order.client_order_id.as_deref() != Some(prepared.client_order_id()) {
        return Err(
            "Submission response client_order_id does not match. Check paper orders before retrying."
                .into(),
        );
    }
    Ok((order, raw))
}

pub fn run_submit(options: SubmitOrderArgs) -> Result<(), String> {
    let prepared = options.prepare()?;

    if !options.confirm {
        print!("{}", dry_run_message(&prepared)?);
        return Ok(());
    }

    // Confirmed requests go only to the fixed Alpaca PAPER host.
    let credentials = load_credentials().map_err(|error| error.to_string())?;
    let (order, original) =
        decode_submit_result(&prepared, fetch_submit_order(&credentials, &prepared))?;

    match &options.output {
        None => {
            println!("Paper order submission returned a response; this does NOT confirm a fill.");
            print!("{}", render_order_summary(&order));
        }
        Some(path) => {
            save_private_json(path, &original)
                .map_err(|_| "Could not save submitted order JSON.".to_owned())?;
            println!("Paper submission response received; fill is NOT guaranteed.");
            println!(
                "Saved complete response JSON privately to {}",
                path.display()
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
