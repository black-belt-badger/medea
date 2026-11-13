mod account;
mod account_details;
mod account_status;
mod asset_types;
mod assets;
mod cli;
mod client;
mod credentials;
mod crypto_bars;
mod crypto_quotes;
mod crypto_trades;
mod order_cancel;
mod order_filters;
mod order_lookup;
mod order_types;
mod orders;
mod output;
mod positions;

use clap::Parser;

use account::{decode_account, render_account_summary};
use assets::{decode_assets, render_assets_summary};
use cli::{Cli, Command, CryptoCommand, OrderCommand};
use client::{fetch_account, fetch_assets, fetch_order, fetch_orders, fetch_positions};
use credentials::load_credentials;
use order_lookup::decode_order_lookup;
use orders::{decode_orders, render_order_summary, render_orders_summary};
use output::save_private_json;
use positions::{decode_positions, render_positions_summary};

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
            // Always validate the response, including for --out.
            let account = decode_account(&response)
                .map_err(|_| "Invalid Alpaca account response.".to_owned())?;
            match options.output {
                None => print!("{}", render_account_summary(&account)),
                Some(path) => {
                    save_private_json(&path, &response)
                        .map_err(|err| format!("Could not save account JSON: {err}"))?;
                    println!("Saved complete account JSON to {}", path.display());
                }
            }
            Ok(())
        }
        Command::Positions(options) => {
            let credentials = load_credentials().map_err(|err| err.to_string())?;
            let response = fetch_positions(&credentials).map_err(|err| err.to_string())?;
            // Always validate the response, including for --out.
            let positions = decode_positions(&response)
                .map_err(|_| "Invalid Alpaca positions response.".to_owned())?;
            match options.output {
                None => print!("{}", render_positions_summary(&positions)),
                Some(path) => {
                    save_private_json(&path, &response)
                        .map_err(|err| format!("Could not save positions JSON: {err}"))?;
                    println!("Saved complete positions JSON to {}", path.display());
                }
            }
            Ok(())
        }
        Command::Order {
            command: OrderCommand::Cancel(options),
        } => order_cancel::run_cancel(options),
        Command::Order {
            command: OrderCommand::Get(options),
        } => {
            let credentials = load_credentials().map_err(|err| err.to_string())?;
            // Decode and verify that the returned primary order ID matches the
            // requested UUID before displaying or exporting *any* bytes.
            let (order, response) =
                decode_order_lookup(&options.order_id, fetch_order(&credentials, &options))?;
            match options.output {
                None => print!("{}", render_order_summary(&order)),
                Some(path) => {
                    save_private_json(&path, &response)
                        .map_err(|err| format!("Could not save order JSON: {err}"))?;
                    println!("Saved complete order JSON to {}", path.display());
                }
            }
            Ok(())
        }
        Command::Orders(options) => {
            options.validate()?;
            let credentials = load_credentials().map_err(|err| err.to_string())?;
            let response = fetch_orders(&credentials, &options).map_err(|err| err.to_string())?;
            // Always validate the provider response, including for --out.
            let orders = decode_orders(&response)
                .map_err(|_| "Invalid Alpaca orders response.".to_owned())?;
            match options.output {
                None => print!("{}", render_orders_summary(&orders)),
                Some(path) => {
                    save_private_json(&path, &response)
                        .map_err(|err| format!("Could not save orders JSON: {err}"))?;
                    println!("Saved complete orders JSON to {}", path.display());
                }
            }
            Ok(())
        }
        Command::Assets(options) => {
            options.validate()?;
            let credentials = load_credentials().map_err(|err| err.to_string())?;
            let response = fetch_assets(&credentials, &options).map_err(|err| err.to_string())?;
            // Validate even when exporting complete provider JSON.
            let assets = decode_assets(&response)
                .map_err(|_| "Invalid Alpaca assets response.".to_owned())?;
            match options.output {
                None => print!("{}", render_assets_summary(&assets)),
                Some(path) => {
                    save_private_json(&path, &response)
                        .map_err(|err| format!("Could not save assets JSON: {err}"))?;
                    println!("Saved complete assets JSON to {}", path.display());
                }
            }
            Ok(())
        }
    }
}
