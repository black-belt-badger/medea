//! Shared synthetic Order contract examples, copied unchanged from Kirke.
//! Every fixture is fabricated test data: no paper account or API keys.

use super::{decode_order, decode_orders, render_order_summary};
use crate::order_types::{
    KnownOrderClass, KnownOrderPositionIntent, KnownOrderSide, KnownOrderState,
    KnownOrderTimeInForce, KnownOrderType, OrderClass, OrderPositionIntent, OrderSide, OrderState,
    OrderTimeInForce, OrderType,
};
use serde_json::{Value, json};

fn fixture(name: &str) -> &'static [u8] {
    match name {
        "equity" => include_bytes!("../tests/fixtures/orders/order-equity.json"),
        "crypto" => include_bytes!("../tests/fixtures/orders/order-crypto.json"),
        "mleg" => include_bytes!("../tests/fixtures/orders/order-mleg.json"),
        "bracket" => include_bytes!("../tests/fixtures/orders/order-bracket.json"),
        "trailing" => include_bytes!("../tests/fixtures/orders/order-trailing.json"),
        "future" => include_bytes!("../tests/fixtures/orders/order-future.json"),
        "mixed" => include_bytes!("../tests/fixtures/orders/orders-mixed.json"),
        "complete" => include_bytes!("../tests/fixtures/orders/order-complete-schema.json"),
        _ => panic!("unknown fixture"),
    }
}

#[test]
fn kirke_shared_contract_pack_decodes_all_six_order_scenarios() {
    let orders = decode_orders(fixture("mixed")).unwrap();
    assert_eq!(orders.len(), 6);
    assert_eq!(
        orders[0].status,
        OrderState::Known(KnownOrderState::PartiallyFilled)
    );
    assert_eq!(orders[1].status, OrderState::Known(KnownOrderState::Filled));
    assert!(orders[2].side.is_none()); // An options multi-leg parent has no side.
    assert_eq!(
        orders[2].details.order_class,
        Some(OrderClass::Known(KnownOrderClass::Mleg))
    );
    assert_eq!(
        orders[3].details.order_class,
        Some(OrderClass::Known(KnownOrderClass::Bracket))
    );
    assert_eq!(
        orders[4].order_type,
        Some(OrderType::Known(KnownOrderType::TrailingStop))
    );
    assert_eq!(orders[5].status, OrderState::Other("FUTURE_STATUS".into()));
}

#[test]
fn partially_filled_equity_order_uses_exact_decimal_strings_and_metadata() {
    let order = decode_order(fixture("equity")).unwrap();
    assert_eq!(order.id, "11111111-2222-4333-8444-555555555555");
    assert_eq!(
        order.client_order_id.as_deref(),
        Some("SYNTHETIC-EQUITY-ORDER-1")
    );
    assert_eq!(order.symbol.as_deref(), Some("AAPL"));
    assert_eq!(order.side, Some(OrderSide::Known(KnownOrderSide::Buy)));
    assert_eq!(
        order.order_type,
        Some(OrderType::Known(KnownOrderType::Limit))
    );
    assert_eq!(order.qty.as_deref(), Some("5.123456789012345678901"));
    assert_eq!(order.filled_qty.as_deref(), Some("2.000000000000000001"));
    assert_eq!(
        order.filled_avg_price.as_deref(),
        Some("192.00000000000000000001")
    );
    assert_eq!(
        order.limit_price.as_deref(),
        Some("193.1234567890123456789")
    );
    assert_eq!(
        order.details.position_intent,
        Some(OrderPositionIntent::Known(
            KnownOrderPositionIntent::BuyToOpen
        ))
    );
    assert_eq!(
        order.details.order_class,
        Some(OrderClass::Known(KnownOrderClass::Simple))
    );
    assert_eq!(
        order.details.time_in_force,
        Some(OrderTimeInForce::Known(KnownOrderTimeInForce::Day))
    );
    assert_eq!(order.details.extended_hours, Some(false));
    assert_eq!(
        order.details.updated_at.as_deref(),
        Some("2026-11-13T00:11:02.123456789Z")
    );
    assert!(order.legs.is_none());
}

#[test]
fn crypto_notional_order_keeps_null_quantity_and_explicit_empty_legs() {
    let order = decode_order(fixture("crypto")).unwrap();
    assert_eq!(order.asset_class.as_deref(), Some("crypto"));
    assert_eq!(order.qty, None);
    assert_eq!(
        order.notional.as_deref(),
        Some("25.1234567890123456789012345")
    );
    assert_eq!(
        order.filled_qty.as_deref(),
        Some("0.000375291823456789012345")
    );
    assert_eq!(
        order.filled_at.as_deref(),
        Some("2026-11-13T01:01:02.000000123Z")
    );
    assert!(matches!(&order.legs, Some(legs) if legs.is_empty()));
}

#[test]
fn multi_leg_options_order_decodes_both_legs_and_renders_ratios() {
    let order = decode_order(fixture("mleg")).unwrap();
    assert!(order.side.is_none());
    assert_eq!(
        order.details.order_class,
        Some(OrderClass::Known(KnownOrderClass::Mleg))
    );
    let legs = order.legs.as_ref().unwrap();
    assert_eq!(legs.len(), 2);
    assert_eq!(legs[0].side, Some(OrderSide::Known(KnownOrderSide::Buy)));
    assert_eq!(legs[1].side, Some(OrderSide::Known(KnownOrderSide::Sell)));
    assert_eq!(
        legs[0].details.ratio_qty.as_deref(),
        Some("1.000000000000000001")
    );
    assert_eq!(
        legs[1].details.ratio_qty.as_deref(),
        Some("2.000000000000000002")
    );
    assert_eq!(legs[0].limit_price.as_deref(), Some("1.2500000001"));
    assert!(legs[0].legs.is_none());

    let summary = render_order_summary(&order);
    assert!(summary.contains("  Legs: 2\n"));
    assert!(summary.contains("ratio 1.000000000000000001"));
    assert!(summary.contains("ratio 2.000000000000000002"));
}

#[test]
fn bracket_stop_leg_and_trailing_stop_values_are_typed() {
    let bracket = decode_order(fixture("bracket")).unwrap();
    assert_eq!(
        bracket.details.order_class,
        Some(OrderClass::Known(KnownOrderClass::Bracket))
    );
    let stop_leg = &bracket.legs.as_ref().unwrap()[0];
    assert_eq!(
        stop_leg.order_type,
        Some(OrderType::Known(KnownOrderType::Stop))
    );
    assert_eq!(stop_leg.stop_price.as_deref(), Some("400.50"));
    assert_eq!(stop_leg.filled_qty.as_deref(), Some("0"));

    let trailing = decode_order(fixture("trailing")).unwrap();
    assert_eq!(trailing.details.trail_price.as_deref(), Some("2.5"));
    assert!(trailing.details.trail_percent.is_none());
    assert_eq!(trailing.details.hwm.as_deref(), Some("252.123456789"));
    assert!(trailing.details.extended_hours.is_none());
}

#[test]
fn unknown_future_enum_values_survive_without_crashing_or_normalization() {
    let order = decode_order(fixture("future")).unwrap();
    assert_eq!(order.status, OrderState::Other("FUTURE_STATUS".into()));
    assert_eq!(order.side, Some(OrderSide::Other("FUTURE_SIDE".into())));
    assert_eq!(
        order.order_type,
        Some(OrderType::Other("FUTURE_TYPE".into()))
    );
    assert_eq!(
        order.details.time_in_force,
        Some(OrderTimeInForce::Other("FUTURE_TIF".into()))
    );
    assert_eq!(order.status.as_str(), "FUTURE_STATUS");
    assert_eq!(order.qty, None);
}

#[test]
fn complete_openapi_contract_fixture_maps_every_documented_order_field() {
    let raw: Value = serde_json::from_slice(fixture("complete")).unwrap();
    let leg = &raw["legs"][0];
    assert!(raw.as_object().unwrap().len() >= 37);
    assert!(leg.as_object().unwrap().len() >= 35);

    let order = decode_order(fixture("complete")).unwrap();
    assert_eq!(order.asset_class.as_deref(), Some("us_option"));
    assert_eq!(order.qty.as_deref(), Some("1.000000000000000000000001"));
    assert_eq!(order.notional, None);
    assert_eq!(
        order.filled_qty.as_deref(),
        Some("0.100000000000000000000001")
    );
    assert_eq!(
        order.filled_avg_price.as_deref(),
        Some("12.000000000000000000000123")
    );
    assert_eq!(
        order.limit_price.as_deref(),
        Some("12.001234567890123456789")
    );
    assert_eq!(
        order.status,
        OrderState::Known(KnownOrderState::PartiallyFilled)
    );
    assert_eq!(order.details.order_type.as_deref(), Some("limit"));
    assert_eq!(
        order.details.position_intent,
        Some(OrderPositionIntent::Known(
            KnownOrderPositionIntent::SellToClose
        ))
    );
    assert_eq!(order.details.group_role.as_deref(), Some("take_profit"));
    assert_eq!(order.details.group_id.as_deref(), Some("synthetic-group"));
    assert_eq!(order.details.hwm.as_deref(), Some("1234.5000"));
    assert_eq!(order.details.extended_hours, Some(false));
    assert_eq!(
        order.details.expires_at.as_deref(),
        Some("2026-11-14T01:02:03.123456789Z")
    );
    assert_eq!(
        order.created_at.as_deref(),
        Some("2026-11-13T01:02:03.123456789Z")
    );
    assert_eq!(
        order.submitted_at.as_deref(),
        Some("2026-11-13T01:02:03.123456789Z")
    );
    assert_eq!(
        order.details.updated_at.as_deref(),
        Some("2026-11-13T01:02:04.123456789Z")
    );

    let leg = &order.legs.as_ref().unwrap()[0];
    assert_eq!(leg.symbol.as_deref(), Some("SYNTH251213P00400000"));
    assert_eq!(leg.details.group_role.as_deref(), Some("stop_loss"));
    assert_eq!(
        leg.details.ratio_qty.as_deref(),
        Some("2.000000000000000002")
    );
    assert!(leg.legs.is_none());
}

#[test]
fn null_empty_zero_and_absent_fields_have_distinct_opaque_raw_json() {
    let mut root = json!({
        "id":"id","status":"new","qty":"0","filled_qty":"0",
        "legs": null, "notional":null
    });
    let order = decode_order(&serde_json::to_vec(&root).unwrap()).unwrap();
    assert_eq!(order.qty.as_deref(), Some("0"));
    assert!(order.legs.is_none());
    assert_eq!(root["notional"], Value::Null);
    root["legs"] = json!([]);
    root["qty"] = Value::Null;
    let order = decode_order(&serde_json::to_vec(&root).unwrap()).unwrap();
    assert_eq!(order.qty, None);
    assert!(matches!(order.legs, Some(ref legs) if legs.is_empty()));
}

#[test]
fn wrong_financial_enum_boolean_and_nested_leg_types_are_rejected() {
    let original: Value = serde_json::from_slice(fixture("complete")).unwrap();
    for (field, bad) in [
        ("ratio_qty", json!(0.000001)),
        ("trail_price", json!(123.45)),
        ("extended_hours", json!("true")),
        ("order_class", json!(42)),
        ("time_in_force", json!(false)),
        ("type", json!(1)),
        ("qty", json!(false)),
        ("position_intent", json!(1)),
    ] {
        let mut wrong = original.clone();
        wrong[field] = bad;
        assert!(
            decode_order(&serde_json::to_vec(&wrong).unwrap()).is_err(),
            "{field}"
        );
    }
    let mut bad = original.clone();
    bad["legs"] = json!("invalid");
    assert!(decode_order(&serde_json::to_vec(&bad).unwrap()).is_err());
    bad["legs"] = json!([null]);
    assert!(decode_order(&serde_json::to_vec(&bad).unwrap()).is_err());
}

#[test]
fn grandchildren_with_orders_are_rejected_but_empty_nested_arrays_are_accepted() {
    let mut root: Value = serde_json::from_slice(fixture("mleg")).unwrap();
    root["legs"][0]["legs"] = json!([{"id":"grandchild","status":"new"}]);
    assert!(decode_order(&serde_json::to_vec(&root).unwrap()).is_err());
    root["legs"][0]["legs"] = json!([]);
    assert!(decode_order(&serde_json::to_vec(&root).unwrap()).is_ok());
}

#[test]
fn empty_known_order_class_and_future_order_class_both_decode() {
    let mut order: Value = serde_json::from_slice(fixture("equity")).unwrap();
    order["order_class"] = json!("");
    assert_eq!(
        decode_order(&serde_json::to_vec(&order).unwrap())
            .unwrap()
            .details
            .order_class,
        Some(OrderClass::Known(KnownOrderClass::Simple)),
    );
    order["order_class"] = json!("FUTURE_CLASS");
    assert_eq!(
        decode_order(&serde_json::to_vec(&order).unwrap())
            .unwrap()
            .details
            .order_class,
        Some(OrderClass::Other("FUTURE_CLASS".into())),
    );
}

#[test]
fn rejects_blank_provider_identity_and_empty_status_side_type() {
    let original: Value = serde_json::from_slice(fixture("equity")).unwrap();
    for (field, bad) in [
        ("id", json!("")),
        ("id", json!("   ")),
        ("status", json!("")),
        ("side", json!("")),
        ("type", json!("")),
    ] {
        let mut wrong = original.clone();
        wrong[field] = bad;
        assert!(
            decode_order(&serde_json::to_vec(&wrong).unwrap()).is_err(),
            "{field}"
        );
    }
}

#[test]
fn raw_fixture_bytes_keep_unknown_large_decimal_strings_exact() {
    let raw = fixture("equity");
    assert!(decode_order(raw).is_ok());
    assert!(
        std::str::from_utf8(raw)
            .unwrap()
            .contains("provider_future_decimal")
    );
    assert!(
        std::str::from_utf8(raw)
            .unwrap()
            .contains("0.123456789012345678901234567890123456789")
    );
    // The commands write the unmodified byte buffer, never serialize Order.
}
