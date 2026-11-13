use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Alpaca's documented integer-valued options trading levels.
///
/// Preserve future integer levels rather than rejecting an otherwise valid
/// account response. A fractional or string level is not valid JSON for this
/// enum and must be rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsLevel {
    Disabled,
    Covered,
    Long,
    Spreads,
    Other(i64),
}

impl OptionsLevel {
    pub fn as_integer(self) -> i64 {
        match self {
            Self::Disabled => 0,
            Self::Covered => 1,
            Self::Long => 2,
            Self::Spreads => 3,
            Self::Other(value) => value,
        }
    }
}

impl From<i64> for OptionsLevel {
    fn from(value: i64) -> Self {
        match value {
            0 => Self::Disabled,
            1 => Self::Covered,
            2 => Self::Long,
            3 => Self::Spreads,
            other => Self::Other(other),
        }
    }
}

impl<'de> Deserialize<'de> for OptionsLevel {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        i64::deserialize(deserializer).map(Self::from)
    }
}

impl Serialize for OptionsLevel {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i64(self.as_integer())
    }
}

/// Fields outside the concise account summary.
///
/// The published OpenAPI account schema requires only `id` and `status`,
/// so every field here is optional. The original provider response remains
/// available unchanged via `account --out`, including fields unknown to us.
///
/// Money and timestamp values are deliberately Strings to preserve the exact
/// spelling/precision provided by Alpaca. No floating-point conversions.
// Fields are validated during decoding; only the concise summary is used by
// the current executable. Future diagnostic commands will read these values.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct AccountDetails {
    pub account_blocked: Option<bool>,
    pub accrued_fees: Option<String>,
    pub balance_asof: Option<String>,
    pub created_at: Option<String>,
    pub initial_margin: Option<String>,
    pub intraday_adjustments: Option<String>,
    pub last_equity: Option<String>,
    pub last_maintenance_margin: Option<String>,
    pub long_market_value: Option<String>,
    pub maintenance_margin: Option<String>,
    pub multiplier: Option<String>,
    pub non_marginable_buying_power: Option<String>,
    pub options_approved_level: Option<OptionsLevel>,
    pub options_buying_power: Option<String>,
    pub options_trading_level: Option<OptionsLevel>,
    pub pending_reg_taf_fees: Option<String>,
    pub pending_transfer_in: Option<String>,
    pub pending_transfer_out: Option<String>,
    pub portfolio_value: Option<String>,
    pub regt_buying_power: Option<String>,
    pub short_market_value: Option<String>,
    pub shorting_enabled: Option<bool>,
    pub sma: Option<String>,
    pub trade_suspended_by_user: Option<bool>,
    pub transfers_blocked: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::OptionsLevel;

    #[test]
    fn options_level_covers_all_known_values_and_preserves_future_integer_levels() {
        for (value, expected) in [
            (0, OptionsLevel::Disabled),
            (1, OptionsLevel::Covered),
            (2, OptionsLevel::Long),
            (3, OptionsLevel::Spreads),
            (7, OptionsLevel::Other(7)),
            (-1, OptionsLevel::Other(-1)),
        ] {
            let json = value.to_string();
            let decoded: OptionsLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, expected);
            assert_eq!(decoded.as_integer(), value);
            assert_eq!(serde_json::to_string(&decoded).unwrap(), json);
        }
    }

    #[test]
    fn options_level_rejects_invalid_json_types() {
        for invalid in ["null", "3.0", "1.5", "\"3\"", "false", "{}", "[]"] {
            assert!(serde_json::from_str::<OptionsLevel>(invalid).is_err());
        }
    }
}
