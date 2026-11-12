mod account;
mod cli;
mod client;
mod credentials;
mod output;
mod positions;

use clap::Parser;

use account::{decode_account, render_account_summary};
use cli::{Cli, Command};
use client::{fetch_account, fetch_positions};
use credentials::load_credentials;
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
    }
}
