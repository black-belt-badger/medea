use serde::{Deserialize, Serialize};

// Incoming API vocabulary is open-ended: known values are typed, while
// unrecognized future strings remain intact through JSON round trips.
// CLI query filters are a separate, intentionally closed set.

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AssetKind {
    Known(KnownAssetKind),
    Other(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnownAssetKind {
    #[serde(rename = "us_equity")]
    UsEquity,
    #[serde(rename = "us_option")]
    UsOption,
    #[serde(rename = "crypto")]
    Crypto,
    #[serde(rename = "crypto_perp")]
    CryptoPerp,
    #[serde(rename = "treasury")]
    Treasury,
    #[serde(rename = "corporate")]
    Corporate,
    #[serde(rename = "global_equity")]
    GlobalEquity,
    #[serde(rename = "us_index")]
    UsIndex,
    #[serde(rename = "us_equity_chain")]
    UsEquityChain,
    #[serde(rename = "ipo")]
    Ipo,
}

impl KnownAssetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UsEquity => "us_equity",
            Self::UsOption => "us_option",
            Self::Crypto => "crypto",
            Self::CryptoPerp => "crypto_perp",
            Self::Treasury => "treasury",
            Self::Corporate => "corporate",
            Self::GlobalEquity => "global_equity",
            Self::UsIndex => "us_index",
            Self::UsEquityChain => "us_equity_chain",
            Self::Ipo => "ipo",
        }
    }
}

impl AssetKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Known(known) => known.as_str(),
            Self::Other(other) => other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AssetVenue {
    Known(KnownAssetVenue),
    Other(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnownAssetVenue {
    #[serde(rename = "AMEX")]
    Amex,
    #[serde(rename = "ARCA")]
    Arca,
    #[serde(rename = "BATS")]
    Bats,
    #[serde(rename = "NYSE")]
    Nyse,
    #[serde(rename = "NASDAQ")]
    Nasdaq,
    #[serde(rename = "NYSEARCA")]
    Nysearca,
    #[serde(rename = "OTC")]
    Otc,
    #[serde(rename = "CRYPTO")]
    Crypto,
    #[serde(rename = "TXSE")]
    Txse,
    #[serde(rename = "ERSX")]
    Ersx,
    #[serde(rename = "CRXL")]
    Crxl,
    #[serde(rename = "ASCX")]
    Ascx,
    #[serde(rename = "INDEX")]
    Index,
    // GET /v2/positions permits an empty exchange string for certain assets.
    #[serde(rename = "")]
    NoExchange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AssetState {
    Known(KnownAssetState),
    Other(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnownAssetState {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "inactive")]
    Inactive,
}

impl KnownAssetState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
        }
    }
}

impl AssetState {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Known(known) => known.as_str(),
            Self::Other(other) => other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AssetAttributeValue {
    Known(KnownAssetAttribute),
    Other(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnownAssetAttribute {
    #[serde(rename = "ptp_no_exception")]
    PtpNoException,
    #[serde(rename = "ptp_with_exception")]
    PtpWithException,
    #[serde(rename = "ipo")]
    Ipo,
    #[serde(rename = "has_options")]
    HasOptions,
    #[serde(rename = "options_late_close")]
    OptionsLateClose,
    #[serde(rename = "fractional_eh_enabled")]
    FractionalEhEnabled,
    #[serde(rename = "overnight_tradable")]
    OvernightTradable,
    #[serde(rename = "overnight_halted")]
    OvernightHalted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BorrowStatus {
    Known(KnownBorrowStatus),
    Other(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnownBorrowStatus {
    #[serde(rename = "easy_to_borrow")]
    EasyToBorrow,
    #[serde(rename = "hard_to_borrow")]
    HardToBorrow,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_provider_enum_domains_preserve_unknown_future_values() {
        for unknown in ["FUTURE_VALUE", "next_value", "unknown"] {
            let json = serde_json::to_string(unknown).unwrap();
            macro_rules! round_trip {
                ($type:ty) => {{
                    let parsed: $type = serde_json::from_str(&json).unwrap();
                    assert_eq!(serde_json::to_string(&parsed).unwrap(), json);
                }};
            }
            round_trip!(AssetKind);
            round_trip!(AssetVenue);
            round_trip!(AssetState);
            round_trip!(AssetAttributeValue);
            round_trip!(BorrowStatus);
        }
    }

    #[test]
    fn known_provider_values_deserialize_as_typed_variants() {
        assert_eq!(
            serde_json::from_str::<AssetKind>(r#""us_index""#).unwrap(),
            AssetKind::Known(KnownAssetKind::UsIndex)
        );
        assert_eq!(
            serde_json::from_str::<AssetVenue>(r#""NYSEARCA""#).unwrap(),
            AssetVenue::Known(KnownAssetVenue::Nysearca)
        );
        assert_eq!(
            serde_json::from_str::<AssetState>(r#""active""#).unwrap(),
            AssetState::Known(KnownAssetState::Active)
        );
        assert_eq!(
            serde_json::from_str::<AssetAttributeValue>(r#""overnight_halted""#).unwrap(),
            AssetAttributeValue::Known(KnownAssetAttribute::OvernightHalted)
        );
        assert_eq!(
            serde_json::from_str::<BorrowStatus>(r#""hard_to_borrow""#).unwrap(),
            BorrowStatus::Known(KnownBorrowStatus::HardToBorrow)
        );
    }

    #[test]
    fn all_documented_asset_classes_decode_as_known_typed_variants() {
        for (wire, expected) in [
            ("us_equity", KnownAssetKind::UsEquity),
            ("us_option", KnownAssetKind::UsOption),
            ("crypto", KnownAssetKind::Crypto),
            ("crypto_perp", KnownAssetKind::CryptoPerp),
            ("treasury", KnownAssetKind::Treasury),
            ("corporate", KnownAssetKind::Corporate),
            ("global_equity", KnownAssetKind::GlobalEquity),
            ("us_index", KnownAssetKind::UsIndex),
            ("us_equity_chain", KnownAssetKind::UsEquityChain),
            ("ipo", KnownAssetKind::Ipo),
        ] {
            let json = serde_json::to_string(wire).unwrap();
            let decoded: AssetKind = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, AssetKind::Known(expected));
            assert_eq!(serde_json::to_string(&decoded).unwrap(), json);
        }
    }

    #[test]
    fn all_thirteen_documented_response_exchanges_decode_as_known_variants() {
        for (wire, expected) in [
            ("AMEX", KnownAssetVenue::Amex),
            ("ARCA", KnownAssetVenue::Arca),
            ("BATS", KnownAssetVenue::Bats),
            ("NYSE", KnownAssetVenue::Nyse),
            ("NASDAQ", KnownAssetVenue::Nasdaq),
            ("NYSEARCA", KnownAssetVenue::Nysearca),
            ("OTC", KnownAssetVenue::Otc),
            ("CRYPTO", KnownAssetVenue::Crypto),
            ("TXSE", KnownAssetVenue::Txse),
            ("ERSX", KnownAssetVenue::Ersx),
            ("CRXL", KnownAssetVenue::Crxl),
            ("ASCX", KnownAssetVenue::Ascx),
            ("INDEX", KnownAssetVenue::Index),
        ] {
            let json = serde_json::to_string(wire).unwrap();
            let decoded: AssetVenue = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, AssetVenue::Known(expected));
            assert_eq!(serde_json::to_string(&decoded).unwrap(), json);
        }
    }

    #[test]
    fn empty_position_exchange_is_a_known_provider_value() {
        let decoded: AssetVenue = serde_json::from_str(r#""""#).unwrap();
        assert_eq!(decoded, AssetVenue::Known(KnownAssetVenue::NoExchange));
        assert_eq!(serde_json::to_string(&decoded).unwrap(), r#""""#);
    }

    #[test]
    fn all_eight_documented_response_attributes_decode_as_known_variants() {
        for (wire, expected) in [
            ("ptp_no_exception", KnownAssetAttribute::PtpNoException),
            ("ptp_with_exception", KnownAssetAttribute::PtpWithException),
            ("ipo", KnownAssetAttribute::Ipo),
            ("has_options", KnownAssetAttribute::HasOptions),
            ("options_late_close", KnownAssetAttribute::OptionsLateClose),
            (
                "fractional_eh_enabled",
                KnownAssetAttribute::FractionalEhEnabled,
            ),
            ("overnight_tradable", KnownAssetAttribute::OvernightTradable),
            ("overnight_halted", KnownAssetAttribute::OvernightHalted),
        ] {
            let json = serde_json::to_string(wire).unwrap();
            let decoded: AssetAttributeValue = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, AssetAttributeValue::Known(expected));
            assert_eq!(serde_json::to_string(&decoded).unwrap(), json);
        }
    }

    #[test]
    fn all_documented_status_and_borrow_values_decode_as_known() {
        for (wire, expected) in [
            ("active", KnownAssetState::Active),
            ("inactive", KnownAssetState::Inactive),
        ] {
            let json = serde_json::to_string(wire).unwrap();
            let decoded: AssetState = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, AssetState::Known(expected));
            assert_eq!(serde_json::to_string(&decoded).unwrap(), json);
        }
        for (wire, expected) in [
            ("easy_to_borrow", KnownBorrowStatus::EasyToBorrow),
            ("hard_to_borrow", KnownBorrowStatus::HardToBorrow),
        ] {
            let json = serde_json::to_string(wire).unwrap();
            let decoded: BorrowStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, BorrowStatus::Known(expected));
            assert_eq!(serde_json::to_string(&decoded).unwrap(), json);
        }
    }

    #[test]
    fn enums_reject_non_string_values() {
        for wrong in ["null", "12", "false", "[]", "{}"] {
            assert!(serde_json::from_str::<AssetKind>(wrong).is_err());
            assert!(serde_json::from_str::<AssetVenue>(wrong).is_err());
            assert!(serde_json::from_str::<AssetState>(wrong).is_err());
            assert!(serde_json::from_str::<AssetAttributeValue>(wrong).is_err());
            assert!(serde_json::from_str::<BorrowStatus>(wrong).is_err());
        }
    }
}
