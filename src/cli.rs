use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser, PartialEq, Eq)]
#[command(
    name = "medea",
    about = "Medea: Alpaca paper trading CLI",
    disable_version_flag = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand, PartialEq, Eq)]
pub enum Command {
    /// Fetch Alpaca account information.
    Account(OutputArgs),
    /// List all open positions in the Alpaca paper account.
    Positions(OutputArgs),
}

#[derive(Debug, Args, PartialEq, Eq)]
pub struct OutputArgs {
    /// Write the complete JSON response to PATH.
    #[arg(long = "out", value_name = "PATH")]
    pub output: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::{Cli, Command, OutputArgs};
    use clap::Parser;

    #[test]
    fn parses_account() {
        let cli = Cli::try_parse_from(["medea", "account"]).unwrap();
        assert_eq!(cli.command, Command::Account(OutputArgs { output: None }));
    }

    #[test]
    fn parses_account_output() {
        let cli = Cli::try_parse_from(["medea", "account", "--out", "account.json"]).unwrap();
        assert_eq!(
            cli.command,
            Command::Account(OutputArgs {
                output: Some("account.json".into())
            })
        );
    }

    #[test]
    fn parses_positions() {
        let cli = Cli::try_parse_from(["medea", "positions"]).unwrap();
        assert_eq!(cli.command, Command::Positions(OutputArgs { output: None }));
    }

    #[test]
    fn parses_positions_output() {
        let cli = Cli::try_parse_from(["medea", "positions", "--out", "positions.json"]).unwrap();
        assert_eq!(
            cli.command,
            Command::Positions(OutputArgs {
                output: Some("positions.json".into())
            })
        );
    }

    #[test]
    fn rejects_missing_positions_output_path() {
        assert!(Cli::try_parse_from(["medea", "positions", "--out"]).is_err());
    }

    #[test]
    fn rejects_unknown_positions_options() {
        assert!(Cli::try_parse_from(["medea", "positions", "--unexpected"]).is_err());
    }

    #[test]
    fn rejects_missing_output_path() {
        assert!(Cli::try_parse_from(["medea", "account", "--out"]).is_err());
    }

    #[test]
    fn rejects_unknown_options() {
        assert!(Cli::try_parse_from(["medea", "account", "--unexpected"]).is_err());
    }

    #[test]
    fn rejects_unknown_subcommands() {
        assert!(Cli::try_parse_from(["medea", "unknown"]).is_err());
    }

    #[test]
    fn requires_a_subcommand() {
        assert!(Cli::try_parse_from(["medea"]).is_err());
    }
}
