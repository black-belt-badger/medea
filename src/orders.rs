use serde::Deserialize;

// Preserve every financial quantity and price exactly as supplied by Alpaca.
// The original provider JSON is separately retained for --out.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Order {
    id: String,
    client_order_id: String,
    symbol: String,
    asset_class: Option<String>,
    side: String,
    #[serde(rename = "type")]
    order_type: String,
    status: String,
    qty: Option<String>,
    notional: Option<String>,
    filled_qty: String,
    filled_avg_price: Option<String>,
    limit_price: Option<String>,
    stop_price: Option<String>,
    created_at: String,
    submitted_at: Option<String>,
    filled_at: Option<String>,
}

pub fn decode_orders(body: &[u8]) -> Result<Vec<Order>, serde_json::Error> {
    serde_json::from_slice(body)
}

pub fn render_orders_summary(orders: &[Order]) -> String {
    let mut summary = String::from("Medea — Alpaca paper orders\n");
    if orders.is_empty() {
        summary.push_str("No matching orders.\n");
        return summary;
    }

    summary.push_str(&format!("Orders: {}\n", orders.len()));
    for order in orders {
        summary.push_str(&format!(
            concat!(
                "{} ({}, {} {}, {})\n",
                "  ID: {}\n",
                "  Client order ID: {}\n",
                "  Quantity: {}\n",
                "  Notional: {}\n",
                "  Filled quantity: {}\n",
                "  Filled average price: {}\n",
                "  Limit price: {}\n",
                "  Stop price: {}\n",
                "  Created at: {}\n",
                "  Submitted at: {}\n",
                "  Filled at: {}\n"
            ),
            order.symbol,
            or_missing(order.asset_class.as_deref()),
            order.side,
            order.order_type,
            order.status,
            order.id,
            order.client_order_id,
            or_missing(order.qty.as_deref()),
            or_missing(order.notional.as_deref()),
            order.filled_qty,
            or_missing(order.filled_avg_price.as_deref()),
            or_missing(order.limit_price.as_deref()),
            or_missing(order.stop_price.as_deref()),
            order.created_at,
            or_missing(order.submitted_at.as_deref()),
            or_missing(order.filled_at.as_deref()),
        ));
    }
    summary
}

fn or_missing(value: Option<&str>) -> &str {
    value.unwrap_or("not provided")
}

#[cfg(test)]
mod tests {
    use super::{decode_orders, render_orders_summary};

    const RESPONSE: &str = r#"[
        {
            "id":"uuid-crypto", "client_order_id":"cid-crypto", "symbol":"BTCUSD",
            "asset_class":"crypto", "side":"buy", "type":"market", "status":"filled",
            "qty":"0.000000000123456789", "notional":null,
            "filled_qty":"0.000000000123456789",
            "filled_avg_price":"90000.123456789", "limit_price":null, "stop_price":null,
            "created_at":"2026-11-12T22:00:00Z",
            "submitted_at":"2026-11-12T22:00:00Z",
            "filled_at":"2026-11-12T22:00:01Z",
            "provider_future_field":{"ignored":true}
        },
        {
            "id":"uuid-equity", "client_order_id":"cid-equity", "symbol":"AAPL",
            "asset_class":"us_equity", "side":"sell", "type":"limit", "status":"new",
            "qty":null, "notional":"123.45000000", "filled_qty":"0",
            "filled_avg_price":null, "limit_price":"203.50",
            "created_at":"2026-11-12T23:00:00Z"
        }
    ]"#;

    #[test]
    fn decodes_crypto_and_equity_and_preserves_decimal_strings() {
        let orders = decode_orders(RESPONSE.as_bytes()).unwrap();
        assert_eq!(orders.len(), 2);
        assert_eq!(orders[0].symbol, "BTCUSD");
        assert_eq!(orders[0].side, "buy");
        assert_eq!(orders[0].order_type, "market");
        assert_eq!(orders[0].status, "filled");
        assert_eq!(orders[0].qty.as_deref(), Some("0.000000000123456789"));
        assert_eq!(orders[0].filled_qty, "0.000000000123456789");
        assert_eq!(
            orders[0].filled_avg_price.as_deref(),
            Some("90000.123456789")
        );
        assert_eq!(orders[1].symbol, "AAPL");
        assert_eq!(orders[1].notional.as_deref(), Some("123.45000000"));
        assert_eq!(orders[1].limit_price.as_deref(), Some("203.50"));
    }

    #[test]
    fn empty_order_history_is_valid_and_clear() {
        let orders = decode_orders(b"[]").unwrap();
        assert!(orders.is_empty());
        assert_eq!(
            render_orders_summary(&orders),
            "Medea — Alpaca paper orders\nNo matching orders.\n"
        );
    }

    #[test]
    fn missing_nullable_fields_are_allowed() {
        let orders = decode_orders(
            br#"[{"id":"id","client_order_id":"cid","symbol":"SOLUSD","side":"buy","type":"market","status":"new","filled_qty":"0","created_at":"2026-11-13T00:00:00Z","qty":null,"submitted_at":null}]"#,
        )
        .unwrap();
        assert_eq!(orders[0].asset_class, None);
        assert_eq!(orders[0].qty, None);
        assert_eq!(orders[0].notional, None);
        assert_eq!(orders[0].submitted_at, None);
        assert_eq!(orders[0].filled_at, None);
        let summary = render_orders_summary(&orders);
        assert!(summary.contains("  Notional: not provided\n"));
        assert!(summary.contains("  Filled at: not provided\n"));
    }

    #[test]
    fn renders_order_metadata_exactly() {
        let summary = render_orders_summary(&decode_orders(RESPONSE.as_bytes()).unwrap());
        assert!(summary.starts_with("Medea — Alpaca paper orders\nOrders: 2\n"));
        assert!(summary.contains("BTCUSD (crypto, buy market, filled)\n"));
        assert!(summary.contains("  ID: uuid-crypto\n"));
        assert!(summary.contains("  Client order ID: cid-crypto\n"));
        assert!(summary.contains("  Quantity: 0.000000000123456789\n"));
        assert!(summary.contains("  Filled average price: 90000.123456789\n"));
        assert!(summary.contains("AAPL (us_equity, sell limit, new)\n"));
        assert!(summary.contains("  Notional: 123.45000000\n"));
    }

    #[test]
    fn rejects_malformed_json_and_wrong_top_level_shape() {
        assert!(decode_orders(b"[broken").is_err());
        assert!(decode_orders(br#"{"id":"single"}"#).is_err());
    }

    #[test]
    fn rejects_missing_required_fields() {
        assert!(decode_orders(br#"[{"symbol":"BTCUSD"}]"#).is_err());
    }

    #[test]
    fn rejects_numeric_quantities_instead_of_rounding() {
        let bad = RESPONSE.replace(
            r#""qty":"0.000000000123456789""#,
            r#""qty":0.000000000123456789"#,
        );
        assert!(decode_orders(bad.as_bytes()).is_err());
        let bad = RESPONSE.replace(
            r#""filled_qty":"0.000000000123456789""#,
            r#""filled_qty":0.000000000123456789"#,
        );
        assert!(decode_orders(bad.as_bytes()).is_err());
    }
}
