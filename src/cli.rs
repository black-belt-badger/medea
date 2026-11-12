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
    Account(AccountArgs),
}

#[derive(Debug, Args, PartialEq, Eq)]
pub struct AccountArgs {
    /// Write the complete JSON response to PATH.
    #[arg(long = "out", value_name = "PATH")]
    pub output: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::{AccountArgs, Cli, Command};
    use clap::Parser;

    #[test]
    fn parses_account() {
        let cli = Cli::try_parse_from(["medea", "account"]).unwrap();
        assert_eq!(cli.command, Command::Account(AccountArgs { output: None }));
    }

    #[test]
    fn parses_account_output() {
        let cli = Cli::try_parse_from(["medea", "account", "--out", "account.json"]).unwrap();
        assert_eq!(
            cli.command,
            Command::Account(AccountArgs {
                output: Some("account.json".into())
            })
        );
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
