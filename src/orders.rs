use serde::{Deserialize, Deserializer};

use crate::order_types::{
    OrderClass, OrderPositionIntent, OrderSide, OrderState, OrderTimeInForce, OrderType,
};

/// Documented order fields that are not part of the concise CLI summary.
/// All decimal amounts and quantities retain Alpaca's exact JSON strings.
#[allow(dead_code)]
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct OrderDetails {
    asset_id: Option<String>,
    position_intent: Option<OrderPositionIntent>,
    // Deprecated provider field, distinct from the typed JSON "type".
    order_type: Option<String>,
    order_class: Option<OrderClass>,
    time_in_force: Option<OrderTimeInForce>,
    trail_price: Option<String>,
    trail_percent: Option<String>,
    hwm: Option<String>,
    extended_hours: Option<bool>,
    updated_at: Option<String>,
    expired_at: Option<String>,
    canceled_at: Option<String>,
    failed_at: Option<String>,
    replaced_at: Option<String>,
    replaced_by: Option<String>,
    replaces: Option<String>,
    expires_at: Option<String>,
    group_id: Option<String>,
    group_role: Option<String>,
    source: Option<String>,
    subtag: Option<String>,
    pub(crate) ratio_qty: Option<String>,
}

/// One paper order, including nullable options/multi-leg attributes.
/// The same model decodes GET /v2/orders and GET /v2/orders/{order_id}.
///
/// The published schema's required list does not match all actual response
/// variants. Match Kirke's tested operational contract: id and status must be
/// present; class-specific and nullable fields are optional. No decimal is
/// converted to floating point. The original provider bytes are saved by
/// --out without reconstruction or loss of future properties.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Order {
    #[serde(deserialize_with = "nonblank_order_id")]
    pub(crate) id: String,
    pub(crate) client_order_id: Option<String>,
    symbol: Option<String>,
    asset_class: Option<String>,
    pub(crate) side: Option<OrderSide>,
    #[serde(rename = "type")]
    order_type: Option<OrderType>,
    pub(crate) status: OrderState,
    pub(crate) qty: Option<String>,
    pub(crate) notional: Option<String>,
    pub(crate) filled_qty: Option<String>,
    filled_avg_price: Option<String>,
    limit_price: Option<String>,
    stop_price: Option<String>,
    created_at: Option<String>,
    submitted_at: Option<String>,
    filled_at: Option<String>,
    #[serde(flatten)]
    pub(crate) details: OrderDetails,
    pub(crate) legs: Option<Vec<Order>>,
}

fn nonblank_order_id<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let id = String::deserialize(deserializer)?;
    if id.trim().is_empty() {
        Err(serde::de::Error::custom("missing nonblank order id"))
    } else {
        Ok(id)
    }
}

/// List and single-order endpoints share the decoder, but not the top-level
/// response shape. A nonempty grandchildren array is not supported in Kirke
/// or Medea's typed CLI; provider extension fields remain in --out JSON.
pub fn decode_order(body: &[u8]) -> Result<Order, String> {
    let order: Order = serde_json::from_slice(body).map_err(|err| err.to_string())?;
    check_leg_depth(&order)?;
    Ok(order)
}

pub fn decode_orders(body: &[u8]) -> Result<Vec<Order>, String> {
    let orders: Vec<Order> = serde_json::from_slice(body).map_err(|err| err.to_string())?;
    for order in &orders {
        check_leg_depth(order)?;
    }
    Ok(orders)
}

fn check_leg_depth(order: &Order) -> Result<(), String> {
    if let Some(legs) = &order.legs {
        for leg in legs {
            if leg
                .legs
                .as_ref()
                .is_some_and(|children| !children.is_empty())
            {
                return Err("Order legs may not nest".into());
            }
        }
    }
    Ok(())
}

pub fn render_orders_summary(orders: &[Order]) -> String {
    let mut summary = String::from("Medea — Alpaca paper orders\n");
    if orders.is_empty() {
        summary.push_str("No matching orders.\n");
        return summary;
    }
    summary.push_str(&format!("Orders: {}\n", orders.len()));
    for order in orders {
        summary.push_str(&render_order(order));
    }
    summary
}

/// Keep a single consistent summary for list and by-ID lookups.
pub fn render_order_summary(order: &Order) -> String {
    let mut summary = String::from("Medea — Alpaca paper order\n");
    summary.push_str(&render_order(order));
    summary
}

fn render_order(order: &Order) -> String {
    let mut summary = format!(
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
        or_missing(order.symbol.as_deref()),
        or_missing(order.asset_class.as_deref()),
        order
            .side
            .as_ref()
            .map_or("not provided", OrderSide::as_str),
        order
            .order_type
            .as_ref()
            .map_or("not provided", OrderType::as_str),
        order.status.as_str(),
        order.id,
        or_missing(order.client_order_id.as_deref()),
        or_missing(order.qty.as_deref()),
        or_missing(order.notional.as_deref()),
        or_missing(order.filled_qty.as_deref()),
        or_missing(order.filled_avg_price.as_deref()),
        or_missing(order.limit_price.as_deref()),
        or_missing(order.stop_price.as_deref()),
        or_missing(order.created_at.as_deref()),
        or_missing(order.submitted_at.as_deref()),
        or_missing(order.filled_at.as_deref()),
    );

    if let Some(legs) = &order.legs
        && !legs.is_empty()
    {
        summary.push_str(&format!("  Legs: {}\n", legs.len()));
        for leg in legs {
            summary.push_str(&format!(
                "    {}: {}, ratio {}, {}\n",
                or_missing(leg.symbol.as_deref()),
                leg.side.as_ref().map_or("not provided", OrderSide::as_str),
                or_missing(leg.details.ratio_qty.as_deref()),
                leg.status.as_str(),
            ));
        }
    }
    summary
}

fn or_missing(value: Option<&str>) -> &str {
    value.unwrap_or("not provided")
}

#[cfg(test)]
mod tests {
    use super::{
        OrderSide, OrderType, decode_order, decode_orders, render_order_summary,
        render_orders_summary,
    };

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
        assert_eq!(orders[0].symbol.as_deref(), Some("BTCUSD"));
        assert_eq!(orders[0].side.as_ref().map(OrderSide::as_str), Some("buy"));
        assert_eq!(
            orders[0].order_type.as_ref().map(OrderType::as_str),
            Some("market")
        );
        assert_eq!(orders[0].status.as_str(), "filled");
        assert_eq!(orders[0].qty.as_deref(), Some("0.000000000123456789"));
        assert_eq!(
            orders[0].filled_qty.as_deref(),
            Some("0.000000000123456789")
        );
        assert_eq!(
            orders[0].filled_avg_price.as_deref(),
            Some("90000.123456789")
        );
        assert_eq!(orders[1].symbol.as_deref(), Some("AAPL"));
        assert_eq!(orders[1].notional.as_deref(), Some("123.45000000"));
        assert_eq!(orders[1].limit_price.as_deref(), Some("203.50"));
    }

    #[test]
    fn decodes_one_order_with_precise_decimals_and_missing_nullable_fields() {
        let response = br#"{
            "id":"id-1","client_order_id":"client-1","symbol":"BTCUSD",
            "side":"buy","type":"market","status":"filled",
            "filled_qty":"0.00000000000001",
            "created_at":"2026-11-13T00:00:00Z",
            "qty":null,"submitted_at":null,"legs":null
        }"#;
        let order = decode_order(response).unwrap();
        assert_eq!(order.id, "id-1");
        assert_eq!(order.symbol.as_deref(), Some("BTCUSD"));
        assert_eq!(order.filled_qty.as_deref(), Some("0.00000000000001"));
        assert_eq!(order.asset_class, None);
        assert_eq!(order.qty, None);
        assert_eq!(order.submitted_at, None);
        assert_eq!(order.filled_at, None);
        let summary = render_order_summary(&order);
        assert!(summary.starts_with("Medea — Alpaca paper order\n"));
        assert!(summary.contains("BTCUSD (not provided, buy market, filled)\n"));
        assert!(summary.contains("  Filled quantity: 0.00000000000001\n"));
        assert!(summary.contains("  Filled at: not provided\n"));
    }

    #[test]
    fn rejects_empty_or_wrong_shape_single_order_response() {
        for response in [
            b"".as_slice(),
            b"[]",
            b"null",
            b"{}",
            b"{invalid",
            br#"{"symbol":"BTCUSD"}"#,
        ] {
            assert!(decode_order(response).is_err());
        }
    }

    #[test]
    fn refuses_numeric_quantities_in_single_order_response() {
        let response = br#"{
            "id":"id","client_order_id":"client","symbol":"SOLUSD",
            "side":"buy","type":"market","status":"new",
            "filled_qty":0.01,"created_at":"2026-11-13T00:00:00Z"
        }"#;
        assert!(decode_order(response).is_err());
    }

    #[test]
    fn single_order_summary_matches_list_fields() {
        let orders = decode_orders(RESPONSE.as_bytes()).unwrap();
        let single = render_order_summary(&orders[0]);
        assert!(single.starts_with("Medea — Alpaca paper order\n"));
        assert!(single.contains("  ID: uuid-crypto\n"));
        assert!(single.contains("  Client order ID: cid-crypto\n"));
        assert!(single.contains("  Quantity: 0.000000000123456789\n"));
        assert!(single.contains("  Filled average price: 90000.123456789\n"));
        assert!(
            render_orders_summary(&orders)
                .contains(&single.replace("Medea — Alpaca paper order\n", ""))
        );
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

#[cfg(test)]
#[path = "orders_fidelity_tests.rs"]
mod fidelity_tests;
