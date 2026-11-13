mod account;
mod account_details;
mod account_findings;
mod account_status;
mod asset_types;
mod assets;
mod cli;
mod client;
mod credentials;
mod crypto_bars;
mod crypto_quotes;
mod crypto_trades;
mod diagnostics;
mod order_cancel;
mod order_filters;
mod order_lookup;
mod order_submit;
mod order_types;
mod orders;
mod output;
mod positions;
mod positions_findings;

use clap::Parser;
use std::io::{self, Write};
use std::path::Path;

use account::{decode_account, render_account_summary};
use account_findings::account_findings;
use assets::{decode_assets, render_assets_summary};
use cli::{Cli, Command, CryptoCommand, OrderCommand, OrderId, ReadFormat};
use client::{
    ClientError, fetch_account, fetch_assets, fetch_order, fetch_orders, fetch_positions,
};
use credentials::load_credentials;
use diagnostics::Finding;
use order_lookup::decode_order_lookup;
use orders::{decode_orders, render_order_summary, render_orders_summary};
use output::save_private_json;
use positions::{decode_positions, render_positions_summary};
use positions_findings::positions_findings;

fn main() {
    if let Err(message) = run(Cli::parse()) {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::Crypto {
            command: CryptoCommand::Bars(options),
        } => crypto_bars::run_bars(options),
        Command::Crypto {
            command: CryptoCommand::Trades(options),
        } => crypto_trades::run_trades(options),
        Command::Crypto {
            command: CryptoCommand::Quotes(options),
        } => crypto_quotes::run_quotes(options),
        Command::Account(options) => {
            let credentials = load_credentials().map_err(|err| err.to_string())?;
            let response = fetch_account(&credentials).map_err(|err| err.to_string())?;
            let (payload, findings) = present_account(
                &response,
                options.output.as_deref(),
                options.format,
                options.check,
            )?;
            if write_cli_stdout(&payload)? {
                for finding in &findings {
                    eprintln!("{}", render_cli_finding(finding));
                }
            }
            Ok(())
        }
        Command::Positions(options) => {
            let credentials = load_credentials().map_err(|err| err.to_string())?;
            let response = fetch_positions(&credentials).map_err(|err| err.to_string())?;
            let (payload, findings) = present_positions(
                &response,
                options.output.as_deref(),
                options.format,
                options.check,
            )?;
            if write_cli_stdout(&payload)? {
                for finding in &findings {
                    eprintln!("{}", render_cli_finding(finding));
                }
            }
            Ok(())
        }
        Command::Order {
            command: OrderCommand::Cancel(options),
        } => order_cancel::run_cancel(options),
        Command::Order {
            command: OrderCommand::Submit(options),
        } => order_submit::run_submit(options),
        Command::Order {
            command: OrderCommand::Get(options),
        } => {
            let credentials = load_credentials().map_err(|err| err.to_string())?;
            let response = fetch_order(&credentials, &options);
            print!(
                "{}",
                present_order_get(&options.order_id, response, options.output.as_deref())?
            );
            Ok(())
        }
        Command::Orders(options) => {
            options.validate()?;
            let credentials = load_credentials().map_err(|err| err.to_string())?;
            let response = fetch_orders(&credentials, &options).map_err(|err| err.to_string())?;
            print!("{}", present_orders(&response, options.output.as_deref())?);
            Ok(())
        }
        Command::Assets(options) => {
            options.validate()?;
            let credentials = load_credentials().map_err(|err| err.to_string())?;
            let response = fetch_assets(&credentials, &options).map_err(|err| err.to_string())?;
            print!("{}", present_assets(&response, options.output.as_deref())?);
            Ok(())
        }
    }
}

// One credential-free acceptance boundary. JSON is byte-exact provider data;
// diagnostics never contaminate stdout, and file mode preserves legacy notices.
fn present_account(
    response: &[u8],
    output: Option<&Path>,
    format: ReadFormat,
    check: bool,
) -> Result<(Vec<u8>, Vec<Finding>), String> {
    let account =
        decode_account(response).map_err(|_| "Invalid Alpaca account response.".to_owned())?;
    let findings = if check {
        account_findings(&account)
    } else {
        Vec::new()
    };
    let payload = match output {
        None => match format {
            ReadFormat::Text => render_account_summary(&account).into_bytes(),
            ReadFormat::Json => response.to_vec(),
        },
        Some(path) => {
            save_private_json(path, response)
                .map_err(|err| format!("Could not save account JSON: {err}"))?;
            format!("Saved complete account JSON to {}\n", path.display()).into_bytes()
        }
    };
    Ok((payload, findings))
}

// Treat an intentionally closed consumer pipe as success, but not unrelated IO.
fn write_cli_stdout(payload: &[u8]) -> Result<bool, String> {
    let stdout = io::stdout();
    let mut output = stdout.lock();
    write_cli_bytes(&mut output, payload)
}

fn write_cli_bytes<W: Write>(output: &mut W, payload: &[u8]) -> Result<bool, String> {
    match output.write_all(payload).and_then(|_| output.flush()) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == io::ErrorKind::BrokenPipe => Ok(false),
        Err(_) => Err("Could not write CLI output.".to_owned()),
    }
}

fn render_cli_finding(finding: &Finding) -> String {
    format!(
        "{} {} ({}): {}",
        finding.severity.as_str().to_uppercase(),
        finding.code,
        finding.path,
        finding.message
    )
}

// Keep assets presentation testable with offline, synthetic provider responses.
// Invalid payloads must not replace a previous file or reach stdout.
fn present_assets(response: &[u8], output: Option<&Path>) -> Result<String, String> {
    let assets =
        decode_assets(response).map_err(|_| "Invalid Alpaca assets response.".to_owned())?;
    match output {
        None => Ok(render_assets_summary(&assets)),
        Some(path) => {
            save_private_json(path, response)
                .map_err(|err| format!("Could not save assets JSON: {err}"))?;
            Ok(format!(
                "Saved complete assets JSON to {}\n",
                path.display()
            ))
        }
    }
}

#[cfg(test)]
#[path = "account_acceptance_tests.rs"]
mod account_acceptance_tests;

#[cfg(test)]
#[path = "account_unix_tests.rs"]
mod account_unix_tests;

// One testable boundary for sensitive portfolio output and validation.
// A malformed upstream payload must never appear on stdout or replace a file.
fn present_positions(
    response: &[u8],
    output: Option<&Path>,
    format: ReadFormat,
    check: bool,
) -> Result<(Vec<u8>, Vec<Finding>), String> {
    let positions =
        decode_positions(response).map_err(|_| "Invalid Alpaca positions response.".to_owned())?;
    let findings = if check {
        positions_findings(&positions)
    } else {
        Vec::new()
    };
    let payload = match output {
        None => match format {
            ReadFormat::Text => render_positions_summary(&positions).into_bytes(),
            ReadFormat::Json => response.to_vec(),
        },
        Some(path) => {
            save_private_json(path, response)
                .map_err(|err| format!("Could not save positions JSON: {err}"))?;
            format!("Saved complete positions JSON to {}\n", path.display()).into_bytes()
        }
    };
    Ok((payload, findings))
}

#[cfg(test)]
#[path = "assets_acceptance_tests.rs"]
mod assets_acceptance_tests;

#[cfg(test)]
#[path = "positions_acceptance_tests.rs"]
mod positions_acceptance_tests;

#[cfg(test)]
#[path = "positions_unix_tests.rs"]
mod positions_unix_tests;

// Validate the complete array before any user-visible output or file export.
// Keep the original provider bytes for lossless explicit JSON exports.
fn present_orders(response: &[u8], output: Option<&Path>) -> Result<String, String> {
    let orders =
        decode_orders(response).map_err(|_| "Invalid Alpaca orders response.".to_owned())?;
    match output {
        None => Ok(render_orders_summary(&orders)),
        Some(path) => {
            save_private_json(path, response)
                .map_err(|err| format!("Could not save orders JSON: {err}"))?;
            Ok(format!(
                "Saved complete orders JSON to {}\n",
                path.display()
            ))
        }
    }
}

#[cfg(test)]
#[path = "orders_acceptance_tests.rs"]
mod orders_acceptance_tests;

// One endpoint boundary for network outcome, UUID matching and safe output.
// Invalid or unrelated provider responses cannot reach stdout or replace files.
fn present_order_get(
    requested: &OrderId,
    response: Result<Vec<u8>, ClientError>,
    output: Option<&Path>,
) -> Result<String, String> {
    let (order, raw) = decode_order_lookup(requested, response)?;
    match output {
        None => Ok(render_order_summary(&order)),
        Some(path) => {
            save_private_json(path, &raw)
                .map_err(|err| format!("Could not save order JSON: {err}"))?;
            Ok(format!("Saved complete order JSON to {}\n", path.display()))
        }
    }
}

#[cfg(test)]
#[path = "order_lookup_acceptance_tests.rs"]
mod order_lookup_acceptance_tests;
