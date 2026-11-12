use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

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
    /// List available Alpaca paper assets (crypto by default).
    Assets(AssetsArgs),
}

#[derive(Debug, Args, PartialEq, Eq)]
pub struct OutputArgs {
    /// Write the complete JSON response to PATH.
    #[arg(long = "out", value_name = "PATH")]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Args, PartialEq, Eq)]
pub struct AssetsArgs {
    /// Asset class: crypto (default) or us_equity.
    #[arg(long = "asset-class", value_enum, default_value_t = AssetClass::Crypto)]
    pub asset_class: AssetClass,
    /// Filter by active, inactive, or all (omitted by default).
    #[arg(long, value_enum)]
    pub status: Option<AssetStatus>,
    /// Write the complete JSON response to PATH.
    #[arg(long = "out", value_name = "PATH")]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum AssetClass {
    Crypto,
    #[value(name = "us_equity")]
    UsEquity,
}

impl AssetClass {
    pub fn as_query(self) -> &'static str {
        match self {
            Self::Crypto => "crypto",
            Self::UsEquity => "us_equity",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum AssetStatus {
    Active,
    Inactive,
    All,
}

impl AssetStatus {
    pub fn as_query(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::All => "all",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AssetClass, AssetStatus, AssetsArgs, Cli, Command, OutputArgs};
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
    fn assets_default_to_explicit_crypto() {
        let cli = Cli::try_parse_from(["medea", "assets"]).unwrap();
        assert_eq!(
            cli.command,
            Command::Assets(AssetsArgs {
                asset_class: AssetClass::Crypto,
                status: None,
                output: None
            })
        );
    }

    #[test]
    fn parses_assets_filters_and_json_output() {
        let cli = Cli::try_parse_from([
            "medea",
            "assets",
            "--asset-class",
            "us_equity",
            "--status",
            "active",
            "--out",
            "assets.json",
        ])
        .unwrap();
        assert_eq!(
            cli.command,
            Command::Assets(AssetsArgs {
                asset_class: AssetClass::UsEquity,
                status: Some(AssetStatus::Active),
                output: Some("assets.json".into())
            })
        );
    }

    #[test]
    fn accepts_all_available_status_filter_values() {
        let cli = Cli::try_parse_from(["medea", "assets", "--status", "all"]).unwrap();
        assert_eq!(
            cli.command,
            Command::Assets(AssetsArgs {
                asset_class: AssetClass::Crypto,
                status: Some(AssetStatus::All),
                output: None
            })
        );
        let cli = Cli::try_parse_from(["medea", "assets", "--status", "inactive"]).unwrap();
        assert_eq!(
            cli.command,
            Command::Assets(AssetsArgs {
                asset_class: AssetClass::Crypto,
                status: Some(AssetStatus::Inactive),
                output: None
            })
        );
    }

    #[test]
    fn rejects_invalid_assets_filters_and_missing_values() {
        for args in [
            vec!["medea", "assets", "--asset-class", "other"],
            vec!["medea", "assets", "--status", "other"],
            vec!["medea", "assets", "--out"],
            vec!["medea", "assets", "--asset-class"],
            vec!["medea", "assets", "--status"],
        ] {
            assert!(Cli::try_parse_from(args).is_err());
        }
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
