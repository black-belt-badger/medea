use std::path::PathBuf;
use std::str::FromStr;

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
    /// List and filter Alpaca paper orders (read-only).
    Orders(OrdersArgs),
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

#[derive(Debug, Args, PartialEq, Eq)]
pub struct OrdersArgs {
    /// Order status: open (default), closed, or all.
    #[arg(long, value_enum, default_value_t = OrderStatus::Open)]
    pub status: OrderStatus,
    /// Maximum orders: 1..500 (defaults to Alpaca's 50).
    #[arg(long, value_name = "N", value_parser = parse_order_limit)]
    pub limit: Option<u16>,
    /// Comma-separated symbols, e.g. BTCUSD,ETHUSD,AAPL.
    #[arg(long, value_name = "SYMBOLS")]
    pub symbols: Option<OrderSymbols>,
    /// Chronological order: asc or desc (default).
    #[arg(long, value_enum, default_value_t = OrderDirection::Desc)]
    pub direction: OrderDirection,
    /// Write the complete unmodified JSON response to PATH.
    #[arg(long = "out", value_name = "PATH")]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OrderStatus {
    Open,
    Closed,
    All,
}

impl OrderStatus {
    pub fn as_query(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::All => "all",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OrderDirection {
    Asc,
    Desc,
}

impl OrderDirection {
    pub fn as_query(self) -> &'static str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderSymbols(pub Vec<String>);

impl FromStr for OrderSymbols {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let symbols: Vec<String> = value.split(',').map(|s| s.trim().to_owned()).collect();
        if symbols.iter().all(|symbol| {
            !symbol.is_empty()
                && symbol
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
        }) {
            Ok(Self(symbols))
        } else {
            Err("Expected comma-separated symbols such as BTCUSD,ETHUSD,AAPL".into())
        }
    }
}

fn parse_order_limit(value: &str) -> Result<u16, String> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err("Order limit must be between 1 and 500".into());
    }
    match value.parse::<u16>() {
        Ok(n @ 1..=500) => Ok(n),
        _ => Err("Order limit must be between 1 and 500".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AssetClass, AssetStatus, AssetsArgs, Cli, Command, OrderDirection, OrderStatus,
        OrderSymbols, OrdersArgs, OutputArgs,
    };
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
    fn orders_default_to_open_desc_with_api_default_limit() {
        let cli = Cli::try_parse_from(["medea", "orders"]).unwrap();
        assert_eq!(
            cli.command,
            Command::Orders(OrdersArgs {
                status: OrderStatus::Open,
                limit: None,
                symbols: None,
                direction: OrderDirection::Desc,
                output: None,
            })
        );
    }

    #[test]
    fn parses_all_order_filters_and_original_json_output() {
        let cli = Cli::try_parse_from([
            "medea",
            "orders",
            "--status",
            "all",
            "--limit",
            "500",
            "--symbols",
            "BTCUSD, ETHUSD,AAPL",
            "--direction",
            "asc",
            "--out",
            "orders.json",
        ])
        .unwrap();
        assert_eq!(
            cli.command,
            Command::Orders(OrdersArgs {
                status: OrderStatus::All,
                limit: Some(500),
                symbols: Some(OrderSymbols(vec![
                    "BTCUSD".into(),
                    "ETHUSD".into(),
                    "AAPL".into()
                ])),
                direction: OrderDirection::Asc,
                output: Some("orders.json".into()),
            })
        );
    }

    #[test]
    fn parses_closed_orders_and_minimum_limit() {
        let cli =
            Cli::try_parse_from(["medea", "orders", "--status", "closed", "--limit", "1"]).unwrap();
        assert_eq!(
            cli.command,
            Command::Orders(OrdersArgs {
                status: OrderStatus::Closed,
                limit: Some(1),
                symbols: None,
                direction: OrderDirection::Desc,
                output: None,
            })
        );
    }

    #[test]
    fn rejects_invalid_order_limits() {
        for limit in [
            "",
            "0",
            "501",
            "-1",
            "1.5",
            "5x",
            " 5",
            "999999999999999999999999",
        ] {
            assert!(
                Cli::try_parse_from(["medea", "orders", "--limit", limit]).is_err(),
                "{limit:?}"
            );
        }
    }

    #[test]
    fn rejects_invalid_order_symbols() {
        for symbols in [
            "",
            "BTCUSD,",
            ",ETHUSD",
            "BTCUSD,,ETHUSD",
            "BTC/USD",
            "btcUSD",
            "BTCUSD&status=all",
        ] {
            assert!(
                Cli::try_parse_from(["medea", "orders", "--symbols", symbols]).is_err(),
                "{symbols:?}"
            );
        }
    }

    #[test]
    fn rejects_unknown_order_filters_and_missing_values() {
        for args in [
            vec!["medea", "orders", "--status", "filled"],
            vec!["medea", "orders", "--direction", "up"],
            vec!["medea", "orders", "--limit"],
            vec!["medea", "orders", "--status"],
            vec!["medea", "orders", "--symbols"],
            vec!["medea", "orders", "--direction"],
            vec!["medea", "orders", "--out"],
            vec!["medea", "orders", "--unknown"],
        ] {
            assert!(Cli::try_parse_from(args).is_err());
        }
    }

    #[test]
    fn parses_single_symbol_with_dot() {
        let cli = Cli::try_parse_from(["medea", "orders", "--symbols", "BRK.B"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Orders(OrdersArgs { symbols: Some(OrderSymbols(ref symbols)), .. }) if symbols == &["BRK.B"]
        ));
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
