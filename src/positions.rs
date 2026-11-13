use serde::{Deserialize, Serialize};

use crate::asset_types::{AssetKind, AssetVenue};

/// Typed long/short side, preserving previously unknown provider strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PositionSide {
    Known(KnownPositionSide),
    Other(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnownPositionSide {
    Long,
    Short,
}

impl PositionSide {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Known(KnownPositionSide::Long) => "long",
            Self::Known(KnownPositionSide::Short) => "short",
            Self::Other(value) => value,
        }
    }
}

/// USD-denominated values for local-currency-trading accounts. If this object
/// exists, Alpaca requires avg_entry_price and cost_basis, nothing else.
#[allow(dead_code)]
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct UsdPositionValues {
    avg_entry_price: String,
    change_today: Option<String>,
    cost_basis: String,
    current_price: Option<String>,
    lastday_price: Option<String>,
    market_value: Option<String>,
    unrealized_intraday_pl: Option<String>,
    unrealized_intraday_plpc: Option<String>,
    unrealized_pl: Option<String>,
    unrealized_plpc: Option<String>,
}

/// All 22 documented GET /v2/positions fields: 17 required and five optional.
///
/// Alpaca sends monetary quantities and fractional P/L ratios as JSON strings,
/// so they remain exact rather than going through floating-point conversion.
/// Unknown response properties survive in the original bytes exported by --out.
#[allow(dead_code)]
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Position {
    asset_class: AssetKind,
    asset_id: String,
    asset_marginable: bool,
    avg_entry_price: String,
    avg_entry_swap_rate: Option<String>,
    change_today: String,
    cost_basis: String,
    current_price: String,
    exchange: AssetVenue,
    lastday_price: String,
    market_value: String,
    prev_swap_rate: Option<String>,
    qty: String,
    qty_available: Option<String>,
    pub(crate) side: PositionSide,
    swap_rate: Option<String>,
    symbol: String,
    unrealized_intraday_pl: String,
    unrealized_intraday_plpc: String,
    unrealized_pl: String,
    unrealized_plpc: String,
    usd: Option<UsdPositionValues>,
}

pub fn decode_positions(body: &[u8]) -> Result<Vec<Position>, serde_json::Error> {
    serde_json::from_slice(body)
}

pub fn render_positions_summary(positions: &[Position]) -> String {
    let mut summary = String::from("Medea — Alpaca paper positions\n");
    if positions.is_empty() {
        summary.push_str("No open positions.\n");
        return summary;
    }

    summary.push_str(&format!("Positions: {}\n", positions.len()));
    for position in positions {
        summary.push_str(&format!(
            concat!(
                "{} ({}, {})\n",
                "  Quantity: {}\n",
                "  Market value: {}\n",
                "  Current price: {}\n",
                "  Unrealized P/L: {}\n"
            ),
            position.symbol,
            position.asset_class.as_str(),
            position.side.as_str(),
            position.qty,
            position.market_value,
            position.current_price,
            position.unrealized_pl,
        ));
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::{KnownPositionSide, PositionSide, decode_positions, render_positions_summary};
    use crate::asset_types::{AssetKind, AssetVenue, KnownAssetKind, KnownAssetVenue};
    use serde_json::{Value, json};

    // Synthetic fixture only; never snapshot private account data.
    fn sample() -> Value {
        json!({
            "asset_class": "crypto",
            "asset_id": "00000000-0000-0000-0000-000000000001",
            "asset_marginable": false,
            "avg_entry_price": "67123.0000000000000000000001",
            "change_today": "-0.00000000001",
            "cost_basis": "68.00123456789012345678",
            "current_price": "67291.12",
            "exchange": "CRYPTO",
            "lastday_price": "66500.50",
            "market_value": "67.123456789012345",
            "qty": "0.000997498123456789012345",
            "side": "long",
            "symbol": "BTCUSD",
            "unrealized_intraday_pl": "-1.00000000000000009",
            "unrealized_intraday_plpc": "-0.0001",
            "unrealized_pl": "-0.000000000001",
            "unrealized_plpc": "-0.0000000000123456789",
            "future_unknown_property": {"precision": "3.1415926535897932384626"}
        })
    }

    fn usd() -> Value {
        json!({
            "avg_entry_price":"67000.000000000000000001",
            "change_today":"-0.123456789",
            "cost_basis":"100.00",
            "current_price":"67123.41",
            "lastday_price":"67120.00",
            "market_value":"99.9999999999999999999",
            "unrealized_intraday_pl":"-0.001",
            "unrealized_intraday_plpc":"-0.00001",
            "unrealized_pl":"-0.2",
            "unrealized_plpc":"-0.002"
        })
    }

    fn decoded(position: &Value) -> Result<super::Position, serde_json::Error> {
        let mut result = decode_positions(&serde_json::to_vec(&[position]).unwrap())?;
        Ok(result.remove(0))
    }

    #[test]
    fn decodes_all_seventeen_required_properties_with_exact_strings() {
        let position = decoded(&sample()).unwrap();
        assert_eq!(
            position.asset_class,
            AssetKind::Known(KnownAssetKind::Crypto)
        );
        assert_eq!(position.asset_id, "00000000-0000-0000-0000-000000000001");
        assert!(!position.asset_marginable);
        assert_eq!(position.avg_entry_price, "67123.0000000000000000000001");
        assert_eq!(position.change_today, "-0.00000000001");
        assert_eq!(position.cost_basis, "68.00123456789012345678");
        assert_eq!(position.current_price, "67291.12");
        assert_eq!(
            position.exchange,
            AssetVenue::Known(KnownAssetVenue::Crypto)
        );
        assert_eq!(position.lastday_price, "66500.50");
        assert_eq!(position.market_value, "67.123456789012345");
        assert_eq!(position.qty, "0.000997498123456789012345");
        assert_eq!(position.side, PositionSide::Known(KnownPositionSide::Long));
        assert_eq!(position.symbol, "BTCUSD");
        assert_eq!(position.unrealized_intraday_pl, "-1.00000000000000009");
        assert_eq!(position.unrealized_intraday_plpc, "-0.0001");
        assert_eq!(position.unrealized_pl, "-0.000000000001");
        assert_eq!(position.unrealized_plpc, "-0.0000000000123456789");
    }

    #[test]
    fn decodes_all_five_optional_fields_and_every_nested_usd_property() {
        let mut position = sample();
        position["avg_entry_swap_rate"] = json!("1.00000000000000000001");
        position["prev_swap_rate"] = json!("0.99999999999999999998");
        position["qty_available"] = json!("0.0009");
        position["swap_rate"] = json!("0.99999999999999999999");
        position["usd"] = usd();
        let parsed = decoded(&position).unwrap();
        assert_eq!(
            parsed.avg_entry_swap_rate.as_deref(),
            Some("1.00000000000000000001")
        );
        assert_eq!(
            parsed.prev_swap_rate.as_deref(),
            Some("0.99999999999999999998")
        );
        assert_eq!(parsed.qty_available.as_deref(), Some("0.0009"));
        assert_eq!(parsed.swap_rate.as_deref(), Some("0.99999999999999999999"));
        let usd = parsed.usd.unwrap();
        assert_eq!(usd.avg_entry_price, "67000.000000000000000001");
        assert_eq!(usd.change_today.as_deref(), Some("-0.123456789"));
        assert_eq!(usd.cost_basis, "100.00");
        assert_eq!(usd.current_price.as_deref(), Some("67123.41"));
        assert_eq!(usd.lastday_price.as_deref(), Some("67120.00"));
        assert_eq!(usd.market_value.as_deref(), Some("99.9999999999999999999"));
        assert_eq!(usd.unrealized_intraday_pl.as_deref(), Some("-0.001"));
        assert_eq!(usd.unrealized_intraday_plpc.as_deref(), Some("-0.00001"));
        assert_eq!(usd.unrealized_pl.as_deref(), Some("-0.2"));
        assert_eq!(usd.unrealized_plpc.as_deref(), Some("-0.002"));
    }

    #[test]
    fn optional_fields_accept_absent_and_explicit_null_and_preserve_zero() {
        let minimal = decoded(&sample()).unwrap();
        assert!(minimal.avg_entry_swap_rate.is_none());
        assert!(minimal.prev_swap_rate.is_none());
        assert!(minimal.qty_available.is_none());
        assert!(minimal.swap_rate.is_none());
        assert!(minimal.usd.is_none());

        let mut input = sample();
        for key in [
            "avg_entry_swap_rate",
            "prev_swap_rate",
            "qty_available",
            "swap_rate",
            "usd",
        ] {
            input[key] = Value::Null;
        }
        let nulled = decoded(&input).unwrap();
        assert!(nulled.qty_available.is_none());
        assert!(nulled.usd.is_none());

        input["qty_available"] = json!("0");
        input["swap_rate"] = json!("0");
        let with_zero = decoded(&input).unwrap();
        assert_eq!(with_zero.qty_available.as_deref(), Some("0"));
        assert_eq!(with_zero.swap_rate.as_deref(), Some("0"));
    }

    #[test]
    fn nested_usd_requires_only_entry_price_and_cost_basis_when_present() {
        let mut input = sample();
        input["usd"] = json!({"avg_entry_price":"42.00", "cost_basis":"100.00"});
        let usd = decoded(&input).unwrap().usd.unwrap();
        assert_eq!(usd.avg_entry_price, "42.00");
        assert_eq!(usd.cost_basis, "100.00");
        assert!(usd.market_value.is_none());
        for bad in [
            json!({"avg_entry_price":"42.00"}),
            json!({"cost_basis":"100.00"}),
            json!({"avg_entry_price":null, "cost_basis":"100.00"}),
            json!({"avg_entry_price":"42.00", "cost_basis":100}),
            json!({"avg_entry_price":"42.00", "cost_basis":"100.00", "market_value":100}),
        ] {
            input["usd"] = bad;
            assert!(decoded(&input).is_err());
        }
    }

    #[test]
    fn decodes_short_equity_and_documented_empty_exchange() {
        let mut equity = sample();
        equity["asset_class"] = json!("us_equity");
        equity["exchange"] = json!("NASDAQ");
        equity["symbol"] = json!("AAPL");
        equity["side"] = json!("short");
        equity["market_value"] = json!("-400.50");
        equity["asset_marginable"] = json!(true);
        let parsed = decoded(&equity).unwrap();
        assert_eq!(
            parsed.asset_class,
            AssetKind::Known(KnownAssetKind::UsEquity)
        );
        assert_eq!(parsed.exchange, AssetVenue::Known(KnownAssetVenue::Nasdaq));
        assert_eq!(parsed.side, PositionSide::Known(KnownPositionSide::Short));
        assert_eq!(parsed.market_value, "-400.50");
        assert!(parsed.asset_marginable);

        equity["asset_class"] = json!("us_option");
        equity["exchange"] = json!("");
        let option = decoded(&equity).unwrap();
        assert_eq!(
            option.asset_class,
            AssetKind::Known(KnownAssetKind::UsOption)
        );
        assert_eq!(
            option.exchange,
            AssetVenue::Known(KnownAssetVenue::NoExchange)
        );
    }

    #[test]
    fn future_enum_strings_are_preserved_and_rendered() {
        let mut input = sample();
        input["asset_class"] = json!("future_asset_class");
        input["exchange"] = json!("FUTURE_EXCHANGE");
        input["side"] = json!("pending_future");
        let parsed = decoded(&input).unwrap();
        assert_eq!(
            parsed.asset_class,
            AssetKind::Other("future_asset_class".into())
        );
        assert_eq!(parsed.exchange, AssetVenue::Other("FUTURE_EXCHANGE".into()));
        assert_eq!(parsed.side, PositionSide::Other("pending_future".into()));
        assert_eq!(parsed.side.as_str(), "pending_future");
        let summary = render_positions_summary(&[parsed]);
        assert!(summary.contains("BTCUSD (future_asset_class, pending_future)\n"));
    }

    #[test]
    fn position_side_known_and_unknown_values_round_trip() {
        for (wire, expected) in [
            ("long", PositionSide::Known(KnownPositionSide::Long)),
            ("short", PositionSide::Known(KnownPositionSide::Short)),
            ("future", PositionSide::Other("future".into())),
        ] {
            let json = serde_json::to_string(wire).unwrap();
            let decoded: PositionSide = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, expected);
            assert_eq!(decoded.as_str(), wire);
            assert_eq!(serde_json::to_string(&decoded).unwrap(), json);
        }
        for invalid in ["null", "1", "true", "{}", "[]"] {
            assert!(serde_json::from_str::<PositionSide>(invalid).is_err());
        }
    }

    #[test]
    fn rejects_missing_or_null_for_all_seventeen_required_fields() {
        let required = [
            "asset_class",
            "asset_id",
            "asset_marginable",
            "avg_entry_price",
            "change_today",
            "cost_basis",
            "current_price",
            "exchange",
            "lastday_price",
            "market_value",
            "qty",
            "side",
            "symbol",
            "unrealized_intraday_pl",
            "unrealized_intraday_plpc",
            "unrealized_pl",
            "unrealized_plpc",
        ];
        for property in required {
            let mut missing = sample();
            missing.as_object_mut().unwrap().remove(property);
            assert!(decoded(&missing).is_err(), "missing {property} accepted");

            let mut null = sample();
            null[property] = Value::Null;
            assert!(decoded(&null).is_err(), "null {property} accepted");
        }
    }

    #[test]
    fn rejects_wrong_json_types_in_required_and_optional_fields() {
        for (property, wrong) in [
            ("asset_class", json!(12)),
            ("asset_id", json!(12)),
            ("asset_marginable", json!("false")),
            ("avg_entry_price", json!(1.25)),
            ("change_today", json!(false)),
            ("cost_basis", json!(10)),
            ("current_price", json!(1.5)),
            ("exchange", json!(true)),
            ("lastday_price", json!(4)),
            ("market_value", json!(10.50)),
            ("qty", json!(0.125)),
            ("side", json!(false)),
            ("symbol", json!(8)),
            ("unrealized_intraday_pl", json!(-1)),
            ("unrealized_intraday_plpc", json!(0)),
            ("unrealized_pl", json!(1)),
            ("unrealized_plpc", json!(0.01)),
            ("avg_entry_swap_rate", json!(0.1)),
            ("prev_swap_rate", json!(false)),
            ("qty_available", json!(1)),
            ("swap_rate", json!(1.0)),
            ("usd", json!("not an object")),
        ] {
            let mut invalid = sample();
            invalid[property] = wrong.clone();
            assert!(
                decoded(&invalid).is_err(),
                "invalid {property} accepted: {wrong}"
            );
        }
    }

    #[test]
    fn empty_portfolio_is_valid_and_summary_is_backward_compatible() {
        let empty = decode_positions(b"[]").unwrap();
        assert!(empty.is_empty());
        assert_eq!(
            render_positions_summary(&empty),
            "Medea — Alpaca paper positions\nNo open positions.\n"
        );

        let mut equity = sample();
        equity["asset_class"] = json!("us_equity");
        equity["exchange"] = json!("NASDAQ");
        equity["symbol"] = json!("AAPL");
        equity["side"] = json!("short");
        equity["market_value"] = json!("-400.50");
        let response = serde_json::to_vec(&[sample(), equity]).unwrap();
        let positions = decode_positions(&response).unwrap();
        let summary = render_positions_summary(&positions);
        assert!(summary.starts_with("Medea — Alpaca paper positions\nPositions: 2\n"));
        for expected in [
            "BTCUSD (crypto, long)\n",
            "  Quantity: 0.000997498123456789012345\n",
            "  Market value: 67.123456789012345\n",
            "  Current price: 67291.12\n",
            "  Unrealized P/L: -0.000000000001\n",
            "AAPL (us_equity, short)\n",
            "  Market value: -400.50\n",
        ] {
            assert!(summary.contains(expected), "summary missing {expected}");
        }
        assert!(!summary.contains("00000000-0000-0000-0000-000000000001"));
    }

    #[test]
    fn preserves_unmodified_json_bytes_for_private_export_even_with_unknown_properties() {
        let response = serde_json::to_vec(&[sample()]).unwrap();
        assert!(decode_positions(&response).is_ok());
        assert!(
            std::str::from_utf8(&response)
                .unwrap()
                .contains("3.1415926535897932384626")
        );
        // Main sends the original bytes, not a serialized Position, to save_private_json.
    }

    #[test]
    fn rejects_non_array_and_malformed_responses() {
        for data in [
            b"{}".as_slice(),
            b"null",
            b"[oops",
            br#"{"symbol":"BTCUSD"}"#,
        ] {
            assert!(decode_positions(data).is_err());
        }
    }
}
