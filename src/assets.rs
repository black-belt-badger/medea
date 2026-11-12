use serde::Deserialize;

// A small, stable display model. Unknown provider fields remain in the raw
// response for --out; do not silently coerce non-boolean API values.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Asset {
    id: String,
    symbol: String,
    name: Option<String>,
    #[serde(rename = "class")]
    asset_class: String,
    status: String,
    tradable: bool,
    fractionable: Option<bool>,
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
            asset.asset_class,
            asset.status,
            asset.name.as_deref().unwrap_or("not provided"),
            yes_no(asset.tradable),
            asset.fractionable.map(yes_no).unwrap_or("not provided"),
        ));
    }
    summary
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

#[cfg(test)]
mod tests {
    use super::{Asset, decode_assets, render_assets_summary};

    const RESPONSE: &str = r#"[
        {
            "id":"crypto-uuid", "symbol":"BTC/USD", "name":"Bitcoin",
            "class":"crypto", "status":"active", "tradable":true,
            "fractionable":true, "provider_future_field":{"ignored":true}
        },
        {
            "id":"equity-uuid", "symbol":"AAPL", "name":"Apple Inc.",
            "class":"us_equity", "status":"inactive", "tradable":false,
            "fractionable":false
        }
    ]"#;

    #[test]
    fn decodes_crypto_and_equities_and_ignores_unknown_fields() {
        let assets = decode_assets(RESPONSE.as_bytes()).unwrap();
        assert_eq!(
            assets,
            vec![
                Asset {
                    id: "crypto-uuid".into(),
                    symbol: "BTC/USD".into(),
                    name: Some("Bitcoin".into()),
                    asset_class: "crypto".into(),
                    status: "active".into(),
                    tradable: true,
                    fractionable: Some(true),
                },
                Asset {
                    id: "equity-uuid".into(),
                    symbol: "AAPL".into(),
                    name: Some("Apple Inc.".into()),
                    asset_class: "us_equity".into(),
                    status: "inactive".into(),
                    tradable: false,
                    fractionable: Some(false),
                },
            ]
        );
    }

    #[test]
    fn empty_result_is_readable() {
        let assets = decode_assets(b"[]").unwrap();
        assert!(assets.is_empty());
        assert_eq!(
            render_assets_summary(&assets),
            "Medea — Alpaca paper assets\nNo matching assets.\n"
        );
    }

    #[test]
    fn optional_fields_can_be_missing_or_null() {
        let assets = decode_assets(
            br#"[{"id":"uuid","symbol":"SOL/USD","class":"crypto","status":"active","tradable":true,"fractionable":null}]"#,
        )
        .unwrap();
        assert_eq!(assets[0].name, None);
        assert_eq!(assets[0].fractionable, None);
        let summary = render_assets_summary(&assets);
        assert!(summary.contains("Name: not provided\n"));
        assert!(summary.contains("Fractionable: not provided\n"));
    }

    #[test]
    fn renders_all_assets() {
        let summary = render_assets_summary(&decode_assets(RESPONSE.as_bytes()).unwrap());
        assert!(summary.starts_with("Medea — Alpaca paper assets\nAssets: 2\n"));
        assert!(summary.contains("BTC/USD (crypto, active)\n"));
        assert!(summary.contains("Name: Bitcoin\n"));
        assert!(summary.contains("Tradable: yes\n"));
        assert!(summary.contains("Fractionable: yes\n"));
        assert!(summary.contains("AAPL (us_equity, inactive)\n"));
        assert!(summary.contains("Tradable: no\n"));
        assert!(summary.contains("Fractionable: no\n"));
    }

    #[test]
    fn rejects_malformed_json_and_wrong_top_level_shape() {
        assert!(decode_assets(b"[oops").is_err());
        assert!(decode_assets(br#"{"id":"one"}"#).is_err());
    }

    #[test]
    fn rejects_missing_required_fields() {
        assert!(decode_assets(br#"[{"symbol":"BTC/USD"}]"#).is_err());
    }

    #[test]
    fn rejects_non_boolean_tradability() {
        assert!(decode_assets(
            br#"[{"id":"x","symbol":"BTC/USD","class":"crypto","status":"active","tradable":"yes"}]"#
        ).is_err());
    }

    #[test]
    fn requires_class_field_not_asset_class_in_response() {
        assert!(decode_assets(
            br#"[{"id":"x","symbol":"BTC/USD","asset_class":"crypto","status":"active","tradable":true}]"#
        ).is_err());
    }
}
