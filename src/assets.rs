use serde::Deserialize;
use serde_json::Number;

use crate::asset_types::{AssetAttributeValue, AssetKind, AssetState, AssetVenue, BorrowStatus};

/// Every property documented by Alpaca's GET /v2/assets response schema.
///
/// The ten required properties are non-optional. All conditional properties
/// are optional. JSON decimal strings remain exact `String`s, while the
/// deprecated JSON-number margin property uses arbitrary-precision `Number`.
/// Unrecognized provider properties remain intact in the original --out JSON.
#[allow(dead_code)] // Future diagnostics/formatters will use the extra fields.
#[derive(Debug, Deserialize, PartialEq)]
pub struct Asset {
    id: String,
    symbol: String,
    name: String,
    #[serde(rename = "class")]
    asset_class: AssetKind,
    exchange: AssetVenue,
    status: AssetState,
    tradable: bool,
    marginable: bool,
    shortable: bool,
    fractionable: bool,
    attributes: Option<Vec<AssetAttributeValue>>,
    borrow_status: Option<BorrowStatus>,
    cusip: Option<String>,
    maintenance_margin_requirement: Option<Number>,
    margin_requirement_long: Option<String>,
    margin_requirement_short: Option<String>,
    min_order_size: Option<String>,
    min_trade_increment: Option<String>,
    price_increment: Option<String>,
}

pub fn decode_assets(body: &[u8]) -> Result<Vec<Asset>, serde_json::Error> {
    serde_json::from_slice(body)
}

pub fn render_assets_summary(assets: &[Asset]) -> String {
    let mut summary = String::from("Medea — Alpaca paper assets\n");
    if assets.is_empty() {
        summary.push_str("No matching assets.\n");
        return summary;
    }
    summary.push_str(&format!("Assets: {}\n", assets.len()));
    for asset in assets {
        summary.push_str(&format!(
            concat!(
                "{} ({}, {})\n",
                "  Name: {}\n",
                "  Tradable: {}\n",
                "  Fractionable: {}\n"
            ),
            asset.symbol,
            asset.asset_class.as_str(),
            asset.status.as_str(),
            asset.name,
            yes_no(asset.tradable),
            yes_no(asset.fractionable),
        ));
    }
    summary
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

#[cfg(test)]
mod tests {
    use super::{decode_assets, render_assets_summary};
    use crate::asset_types::{
        AssetAttributeValue, AssetKind, AssetState, AssetVenue, BorrowStatus, KnownAssetAttribute,
        KnownAssetKind, KnownAssetState, KnownAssetVenue, KnownBorrowStatus,
    };
    use serde_json::{Value, json};

    const RESPONSE: &str = r#"[
        {
            "id":"crypto-uuid", "symbol":"BTC/USD", "name":"Bitcoin",
            "class":"crypto", "exchange":"CRYPTO", "status":"active",
            "tradable":true, "marginable":false, "shortable":false, "fractionable":true,
            "provider_future_field":{"ignored":true}
        },
        {
            "id":"equity-uuid", "symbol":"AAPL", "name":"Apple Inc.",
            "class":"us_equity", "exchange":"NASDAQ", "status":"inactive",
            "tradable":false, "marginable":true, "shortable":true, "fractionable":false
        }
    ]"#;

    const FULL_SCHEMA_RESPONSE: &str = r#"[
        {
            "id":"equity-id", "class":"us_equity", "exchange":"NYSEARCA",
            "symbol":"SPY", "name":"SPDR S&P 500 ETF Trust", "status":"active",
            "tradable":true, "marginable":true, "shortable":true, "fractionable":true,
            "attributes":["has_options","overnight_tradable"],
            "borrow_status":"easy_to_borrow", "cusip":null,
            "maintenance_margin_requirement":0.30750000000000000000000000000007,
            "margin_requirement_long":"0.305000000000000001",
            "margin_requirement_short":"1.3000",
            "min_order_size":"0.0000000000000000000000001",
            "min_trade_increment":"0.000000000001",
            "price_increment":"0.00005",
            "future_field":{"precision":"123456789012345678900"}
        }
    ]"#;

    #[test]
    fn decodes_crypto_and_equities_and_ignores_unknown_provider_fields() {
        let assets = decode_assets(RESPONSE.as_bytes()).unwrap();
        assert_eq!(assets.len(), 2);
        assert_eq!(assets[0].id, "crypto-uuid");
        assert_eq!(assets[0].symbol, "BTC/USD");
        assert_eq!(assets[0].name, "Bitcoin");
        assert_eq!(
            assets[0].asset_class,
            AssetKind::Known(KnownAssetKind::Crypto)
        );
        assert_eq!(
            assets[0].exchange,
            AssetVenue::Known(KnownAssetVenue::Crypto)
        );
        assert_eq!(assets[0].status, AssetState::Known(KnownAssetState::Active));
        assert!(assets[0].tradable);
        assert!(!assets[0].marginable);
        assert!(!assets[0].shortable);
        assert!(assets[0].fractionable);
        assert_eq!(
            assets[1].asset_class,
            AssetKind::Known(KnownAssetKind::UsEquity)
        );
        assert_eq!(
            assets[1].exchange,
            AssetVenue::Known(KnownAssetVenue::Nasdaq)
        );
        assert_eq!(
            assets[1].status,
            AssetState::Known(KnownAssetState::Inactive)
        );
        assert!(!assets[1].tradable);
        assert!(assets[1].marginable);
        assert!(assets[1].shortable);
        assert!(!assets[1].fractionable);
    }

    #[test]
    fn decodes_all_nineteen_documented_fields_with_exact_numbers() {
        let assets = decode_assets(FULL_SCHEMA_RESPONSE.as_bytes()).unwrap();
        let a = &assets[0];
        assert_eq!(a.id, "equity-id");
        assert_eq!(a.symbol, "SPY");
        assert_eq!(a.name, "SPDR S&P 500 ETF Trust");
        assert_eq!(a.asset_class, AssetKind::Known(KnownAssetKind::UsEquity));
        assert_eq!(a.exchange, AssetVenue::Known(KnownAssetVenue::Nysearca));
        assert_eq!(a.status, AssetState::Known(KnownAssetState::Active));
        assert!(a.tradable);
        assert!(a.marginable);
        assert!(a.shortable);
        assert!(a.fractionable);
        assert_eq!(
            a.attributes,
            Some(vec![
                AssetAttributeValue::Known(KnownAssetAttribute::HasOptions),
                AssetAttributeValue::Known(KnownAssetAttribute::OvernightTradable),
            ])
        );
        assert_eq!(
            a.borrow_status,
            Some(BorrowStatus::Known(KnownBorrowStatus::EasyToBorrow))
        );
        assert_eq!(a.cusip, None);
        assert_eq!(
            a.maintenance_margin_requirement
                .as_ref()
                .unwrap()
                .to_string(),
            "0.30750000000000000000000000000007"
        );
        assert_eq!(
            a.margin_requirement_long.as_deref(),
            Some("0.305000000000000001")
        );
        assert_eq!(a.margin_requirement_short.as_deref(), Some("1.3000"));
        assert_eq!(
            a.min_order_size.as_deref(),
            Some("0.0000000000000000000000001")
        );
        assert_eq!(a.min_trade_increment.as_deref(), Some("0.000000000001"));
        assert_eq!(a.price_increment.as_deref(), Some("0.00005"));
    }

    #[test]
    fn preserves_future_response_enum_values_and_strings() {
        let json = r#"[
          {"id":"id","symbol":"X","name":"Future","class":"new_kind",
           "exchange":"MOON","status":"halted","tradable":false,
           "marginable":false,"shortable":false,"fractionable":false,
           "attributes":["has_options","future_attribute"],"borrow_status":"future_borrow"}
        ]"#;
        let a = &decode_assets(json.as_bytes()).unwrap()[0];
        assert_eq!(a.asset_class, AssetKind::Other("new_kind".into()));
        assert_eq!(a.exchange, AssetVenue::Other("MOON".into()));
        assert_eq!(a.status, AssetState::Other("halted".into()));
        assert_eq!(
            a.attributes,
            Some(vec![
                AssetAttributeValue::Known(KnownAssetAttribute::HasOptions),
                AssetAttributeValue::Other("future_attribute".into()),
            ])
        );
        assert_eq!(
            a.borrow_status,
            Some(BorrowStatus::Other("future_borrow".into()))
        );
        assert_eq!(a.asset_class.as_str(), "new_kind");
        assert_eq!(a.status.as_str(), "halted");
    }

    #[test]
    fn empty_result_has_readable_summary() {
        let assets = decode_assets(b"[]").unwrap();
        assert!(assets.is_empty());
        assert_eq!(
            render_assets_summary(&assets),
            "Medea — Alpaca paper assets\nNo matching assets.\n"
        );
    }

    #[test]
    fn optional_properties_can_be_missing_or_null() {
        let bare = &decode_assets(RESPONSE.as_bytes()).unwrap()[0];
        assert!(bare.attributes.is_none());
        assert!(bare.borrow_status.is_none());
        assert!(bare.cusip.is_none());
        assert!(bare.margin_requirement_short.is_none());
        let explicit = RESPONSE.replace(
            r#""provider_future_field":{"ignored":true}"#,
            r#""attributes":null,"borrow_status":null,"cusip":null,"price_increment":null"#,
        );
        let a = &decode_assets(explicit.as_bytes()).unwrap()[0];
        assert!(a.attributes.is_none());
        assert!(a.borrow_status.is_none());
        assert!(a.cusip.is_none());
        assert!(a.price_increment.is_none());
    }

    #[test]
    fn summary_keeps_existing_human_contract_and_omits_ids() {
        let summary = render_assets_summary(&decode_assets(RESPONSE.as_bytes()).unwrap());
        assert!(summary.starts_with("Medea — Alpaca paper assets\nAssets: 2\n"));
        for expected in [
            "BTC/USD (crypto, active)\n",
            "Name: Bitcoin\n",
            "Tradable: yes\n",
            "Fractionable: yes\n",
            "AAPL (us_equity, inactive)\n",
            "Tradable: no\n",
            "Fractionable: no\n",
        ] {
            assert!(summary.contains(expected), "summary missing {expected}");
        }
        assert!(!summary.contains("crypto-uuid"));
    }

    #[test]
    fn rejects_missing_or_null_for_each_required_response_property() {
        let required = [
            "id",
            "class",
            "exchange",
            "symbol",
            "name",
            "status",
            "tradable",
            "marginable",
            "shortable",
            "fractionable",
        ];
        let first: Value = serde_json::from_str(RESPONSE).unwrap();
        for property in required {
            let mut missing = first.clone();
            missing[0].as_object_mut().unwrap().remove(property);
            assert!(
                decode_assets(&serde_json::to_vec(&missing).unwrap()).is_err(),
                "Missing required property {property} was accepted"
            );

            let mut null = first.clone();
            null[0][property] = Value::Null;
            assert!(
                decode_assets(&serde_json::to_vec(&null).unwrap()).is_err(),
                "Null required property {property} was accepted"
            );
        }
    }

    #[test]
    fn rejects_wrong_types_for_all_response_field_shapes() {
        let first: Value = serde_json::from_str(RESPONSE).unwrap();
        for (key, wrong) in [
            ("class", json!(42)),
            ("exchange", json!(true)),
            ("status", json!({})),
            ("tradable", json!("yes")),
            ("marginable", json!(0)),
            ("shortable", json!("false")),
            ("fractionable", json!(1)),
            ("name", json!(null)),
            ("attributes", json!("has_options")),
            ("attributes", json!([42])),
            ("borrow_status", json!(false)),
            ("cusip", json!(4)),
            ("maintenance_margin_requirement", json!("0.35")),
            ("margin_requirement_long", json!(0.25)),
            ("margin_requirement_short", json!(0.25)),
            ("min_order_size", json!(1)),
            ("min_trade_increment", json!(1)),
            ("price_increment", json!(1)),
        ] {
            let mut invalid = first.clone();
            invalid[0][key] = wrong.clone();
            assert!(
                decode_assets(&serde_json::to_vec(&invalid).unwrap()).is_err(),
                "Wrong type for {key} accepted: {wrong}"
            );
        }
    }

    #[test]
    fn rejects_non_array_and_malformed_json() {
        for data in [b"{}".as_slice(), b"null", b"[oops", br#"{"id":"id"}"#] {
            assert!(decode_assets(data).is_err());
        }
    }

    #[test]
    fn rejects_asset_class_query_key_masquerading_as_response_class() {
        let response = RESPONSE.replace(r#""class":"crypto""#, r#""asset_class":"crypto""#);
        assert!(decode_assets(response.as_bytes()).is_err());
    }

    #[test]
    fn retains_original_response_bytes_for_raw_export() {
        let bytes = FULL_SCHEMA_RESPONSE.as_bytes();
        assert!(decode_assets(bytes).is_ok());
        assert!(
            std::str::from_utf8(bytes)
                .unwrap()
                .contains("123456789012345678900")
        );
        // Main sends these very same bytes to save_private_json (--out).
    }
}
