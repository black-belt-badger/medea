use std::str::FromStr;

use clap::ValueEnum;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::cli::{AssetClass, OrdersArgs};

/// Exclusive order submission-time bound. Retain the caller's exact RFC3339
/// spelling (offset and fractional precision), but compare absolute instants.
/// Historical market-data date-only shorthand is not valid for orders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderInstant {
    raw: String,
    absolute: OffsetDateTime,
}

impl OrderInstant {
    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

impl FromStr for OrderInstant {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if !value.contains('T') {
            return Err("Order timestamp must be timezone-qualified RFC3339".into());
        }
        let absolute = OffsetDateTime::parse(value, &Rfc3339)
            .map_err(|_| "Order timestamp must be timezone-qualified RFC3339")?;
        Ok(Self {
            raw: value.to_owned(),
            absolute,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OrderSideFilter {
    Buy,
    Sell,
}

impl OrderSideFilter {
    pub fn as_query(self) -> &'static str {
        match self {
            Self::Buy => "buy",
            Self::Sell => "sell",
        }
    }
}

/// `all` is a special list-order filter, not an Alpaca asset class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderAssetClass {
    Class(AssetClass),
    All,
}

impl OrderAssetClass {
    pub fn as_query(self) -> &'static str {
        match self {
            Self::Class(class) => class.as_query(),
            Self::All => "all",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderAssetClasses(pub Vec<OrderAssetClass>);

impl OrderAssetClasses {
    pub fn as_query(&self) -> String {
        self.0
            .iter()
            .map(|class| class.as_query())
            .collect::<Vec<_>>()
            .join(",")
    }
}

impl FromStr for OrderAssetClasses {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut classes = Vec::new();
        for token in value.split(',').map(str::trim) {
            if token.is_empty() {
                return Err("Expected nonempty comma-separated order asset classes".into());
            }
            let class = if token == "all" {
                OrderAssetClass::All
            } else {
                OrderAssetClass::Class(
                    <AssetClass as ValueEnum>::from_str(token, false)
                        .map_err(|_| format!("Unknown order asset class: {token}"))?,
                )
            };
            if classes.contains(&class) {
                return Err(format!("Duplicate order asset class: {token}"));
            }
            classes.push(class);
        }
        if classes.len() > 1 && classes.contains(&OrderAssetClass::All) {
            return Err("asset_class=all cannot be combined with individual classes".into());
        }
        Ok(Self(classes))
    }
}

impl OrdersArgs {
    /// Reject unsafe/incompatible combinations before credentials and IO.
    pub fn validate(&self) -> Result<(), String> {
        if let (Some(start), Some(end)) = (&self.after, &self.until)
            && start.absolute > end.absolute
        {
            return Err("--after must not be later than --until".into());
        }
        let has_anchor = self.before_order_id.is_some() || self.after_order_id.is_some();
        if has_anchor && (self.after.is_some() || self.until.is_some()) {
            return Err("Order-ID pagination cannot be combined with --after or --until".into());
        }
        if self.before_order_id.is_some() && self.after_order_id.is_some() {
            return Err("--before-order-id and --after-order-id are mutually exclusive".into());
        }
        if self
            .asset_classes
            .as_ref()
            .is_some_and(|classes| classes.0.is_empty())
        {
            return Err("asset_class filter must not be empty".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_timestamps_keep_fractional_precision_and_original_timezone() {
        for input in [
            "2026-11-12T01:02:03Z",
            "2026-11-12T01:02:03.123456789Z",
            "2026-11-12T02:02:03.123456789+01:00",
        ] {
            assert_eq!(input.parse::<OrderInstant>().unwrap().as_str(), input);
        }
        let a: OrderInstant = "2026-11-12T01:02:03.123456789Z".parse().unwrap();
        let b: OrderInstant = "2026-11-12T02:02:03.123456789+01:00".parse().unwrap();
        assert_eq!(a.absolute, b.absolute);
    }

    #[test]
    fn order_timestamp_rejects_date_only_unqualified_or_invalid_time() {
        for input in [
            "",
            "2026-11-12",
            "yesterday",
            "2026-11-12T01:02:03",
            "2026-11-12T99:02:03Z",
            "2026-11-12T01:02:03Z&limit=500",
        ] {
            assert!(input.parse::<OrderInstant>().is_err(), "{input}");
        }
    }

    #[test]
    fn all_ten_asset_classes_and_all_are_accepted() {
        for (input, class) in [
            ("us_equity", AssetClass::UsEquity),
            ("us_option", AssetClass::UsOption),
            ("crypto", AssetClass::Crypto),
            ("crypto_perp", AssetClass::CryptoPerp),
            ("treasury", AssetClass::Treasury),
            ("corporate", AssetClass::Corporate),
            ("global_equity", AssetClass::GlobalEquity),
            ("us_index", AssetClass::UsIndex),
            ("us_equity_chain", AssetClass::UsEquityChain),
            ("ipo", AssetClass::Ipo),
        ] {
            let parsed: OrderAssetClasses = input.parse().unwrap();
            assert_eq!(
                parsed,
                OrderAssetClasses(vec![OrderAssetClass::Class(class)])
            );
            assert_eq!(parsed.as_query(), input);
        }
        assert_eq!(
            "all".parse::<OrderAssetClasses>().unwrap(),
            OrderAssetClasses(vec![OrderAssetClass::All]),
        );
    }

    #[test]
    fn csv_classes_trim_spaces_and_reject_unknown_duplicate_or_all_mixtures() {
        let classes: OrderAssetClasses = "crypto, us_equity,us_option".parse().unwrap();
        assert_eq!(classes.as_query(), "crypto,us_equity,us_option");
        for invalid in [
            "",
            ",",
            "crypto,",
            ",crypto",
            "crypto,,us_equity",
            "crypto,crypto",
            "all,crypto",
            "us_equity,all",
            "BTCUSD",
            "CRYPTO",
            "crypto%2Cus_equity",
        ] {
            assert!(invalid.parse::<OrderAssetClasses>().is_err(), "{invalid}");
        }
    }
    fn sample_orders() -> OrdersArgs {
        use crate::cli::{OrderDirection, OrderStatus};
        OrdersArgs {
            status: OrderStatus::Open,
            limit: None,
            symbols: None,
            direction: OrderDirection::Desc,
            output: None,
            after: None,
            until: None,
            nested: false,
            side: None,
            asset_classes: None,
            before_order_id: None,
            after_order_id: None,
        }
    }

    #[test]
    fn side_filter_uses_only_buy_and_sell_spelling() {
        assert_eq!(OrderSideFilter::Buy.as_query(), "buy");
        assert_eq!(OrderSideFilter::Sell.as_query(), "sell");
    }

    #[test]
    fn order_bounds_compare_absolute_instants_across_timezones() {
        let valid = OrdersArgs {
            after: Some("2026-11-12T00:00:00Z".parse().unwrap()),
            until: Some("2026-11-12T02:00:00+01:00".parse().unwrap()),
            ..sample_orders()
        };
        assert_eq!(valid.validate(), Ok(()));

        let equal = OrdersArgs {
            after: Some("2026-11-12T01:00:00+01:00".parse().unwrap()),
            until: Some("2026-11-12T00:00:00Z".parse().unwrap()),
            ..sample_orders()
        };
        assert_eq!(equal.validate(), Ok(()));

        let reversed = OrdersArgs {
            after: Some("2026-11-12T02:00:00Z".parse().unwrap()),
            until: Some("2026-11-12T01:00:00Z".parse().unwrap()),
            ..sample_orders()
        };
        assert!(reversed.validate().is_err());
    }

    #[test]
    fn order_pagination_rejects_both_uuid_anchors_together() {
        let options = OrdersArgs {
            before_order_id: Some("9275f66a-0a1b-4a22-8c55-167416c7cf14".parse().unwrap()),
            after_order_id: Some("b27d45fe-0a1b-4a22-8c55-167416c7cf14".parse().unwrap()),
            ..sample_orders()
        };
        assert!(options.validate().is_err());
    }

    #[test]
    fn either_uuid_anchor_conflicts_with_either_date_bound() {
        let id: crate::cli::OrderId = "9275f66a-0a1b-4a22-8c55-167416c7cf14".parse().unwrap();
        let timestamp: OrderInstant = "2026-11-12T01:00:00Z".parse().unwrap();

        for options in [
            OrdersArgs {
                before_order_id: Some(id.clone()),
                after: Some(timestamp.clone()),
                ..sample_orders()
            },
            OrdersArgs {
                before_order_id: Some(id.clone()),
                until: Some(timestamp.clone()),
                ..sample_orders()
            },
            OrdersArgs {
                after_order_id: Some(id.clone()),
                after: Some(timestamp.clone()),
                ..sample_orders()
            },
            OrdersArgs {
                after_order_id: Some(id.clone()),
                until: Some(timestamp.clone()),
                ..sample_orders()
            },
        ] {
            assert!(options.validate().is_err());
        }
    }

    #[test]
    fn each_order_cursor_is_valid_independently() {
        let id: crate::cli::OrderId = "9275f66a-0a1b-4a22-8c55-167416c7cf14".parse().unwrap();
        let before = OrdersArgs {
            before_order_id: Some(id.clone()),
            side: Some(OrderSideFilter::Buy),
            ..sample_orders()
        };
        let after = OrdersArgs {
            after_order_id: Some(id),
            nested: true,
            ..sample_orders()
        };
        assert_eq!(before.validate(), Ok(()));
        assert_eq!(after.validate(), Ok(()));
    }

    #[test]
    fn programmatically_empty_class_filter_is_rejected() {
        let options = OrdersArgs {
            asset_classes: Some(OrderAssetClasses(vec![])),
            ..sample_orders()
        };
        assert!(options.validate().is_err());
    }
}
