use serde::Deserialize;

// Alpaca specifies decimal quantities and prices as JSON strings. Keep them
// exact instead of converting them into floating-point values.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Position {
    symbol: String,
    asset_class: String,
    side: String,
    qty: String,
    market_value: Option<String>,
    current_price: Option<String>,
    unrealized_pl: Option<String>,
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
            position.asset_class,
            position.side,
            position.qty,
            position.market_value.as_deref().unwrap_or("not provided"),
            position.current_price.as_deref().unwrap_or("not provided"),
            position.unrealized_pl.as_deref().unwrap_or("not provided"),
        ));
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::{Position, decode_positions, render_positions_summary};

    const RESPONSE: &str = r#"[
        {
            "symbol":"BTCUSD", "asset_class":"crypto", "side":"long",
            "qty":"0.000997498123456789", "market_value":"67.123456789012345",
            "current_price":"67291.12", "unrealized_pl":"-0.000000000001",
            "provider_future_field":{"ignored":true}
        },
        {
            "symbol":"AAPL", "asset_class":"us_equity", "side":"short",
            "qty":"2", "market_value":"-400.50",
            "current_price":"200.25", "unrealized_pl":"50.50"
        }
    ]"#;

    #[test]
    fn decodes_crypto_and_equity_with_exact_decimal_strings() {
        let positions = decode_positions(RESPONSE.as_bytes()).unwrap();
        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0].symbol, "BTCUSD");
        assert_eq!(positions[0].qty, "0.000997498123456789");
        assert_eq!(
            positions[0].market_value.as_deref(),
            Some("67.123456789012345")
        );
        assert_eq!(
            positions[0].unrealized_pl.as_deref(),
            Some("-0.000000000001")
        );
        assert_eq!(positions[1].asset_class, "us_equity");
        assert_eq!(positions[1].side, "short");
    }

    #[test]
    fn empty_portfolio_is_valid_and_readable() {
        let positions = decode_positions(b"[]").unwrap();
        assert_eq!(positions, Vec::<Position>::new());
        assert_eq!(
            render_positions_summary(&positions),
            "Medea — Alpaca paper positions\nNo open positions.\n"
        );
    }

    #[test]
    fn missing_and_null_optional_fields_are_allowed() {
        let positions = decode_positions(
            br#"[{"symbol":"SOLUSD","asset_class":"crypto","side":"long","qty":"0.125","market_value":null}]"#,
        )
        .unwrap();
        assert_eq!(positions[0].market_value, None);
        assert_eq!(positions[0].current_price, None);
        assert_eq!(positions[0].unrealized_pl, None);
        assert!(render_positions_summary(&positions).contains("Current price: not provided\n"));
    }

    #[test]
    fn renders_all_fields_without_rounding() {
        let positions = decode_positions(RESPONSE.as_bytes()).unwrap();
        let summary = render_positions_summary(&positions);
        assert!(summary.starts_with("Medea — Alpaca paper positions\nPositions: 2\n"));
        assert!(summary.contains("BTCUSD (crypto, long)\n"));
        assert!(summary.contains("  Quantity: 0.000997498123456789\n"));
        assert!(summary.contains("  Market value: 67.123456789012345\n"));
        assert!(summary.contains("  Unrealized P/L: -0.000000000001\n"));
        assert!(summary.contains("AAPL (us_equity, short)\n"));
    }

    #[test]
    fn rejects_invalid_json_and_wrong_top_level_shape() {
        assert!(decode_positions(b"[oops").is_err());
        assert!(decode_positions(br#"{"symbol":"BTCUSD"}"#).is_err());
    }

    #[test]
    fn rejects_missing_required_fields() {
        assert!(
            decode_positions(br#"[{"qty":"1","side":"long","asset_class":"crypto"}]"#).is_err()
        );
    }

    #[test]
    fn rejects_numeric_quantities_and_monetary_values() {
        assert!(
            decode_positions(
                br#"[{"symbol":"BTCUSD","asset_class":"crypto","side":"long","qty":0.125}]"#
            )
            .is_err()
        );
        assert!(
            decode_positions(
                br#"[{"symbol":"BTCUSD","asset_class":"crypto","side":"long","qty":"0.125","market_value":10.50}]"#
            )
            .is_err()
        );
    }
}
