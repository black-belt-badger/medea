use serde::de::{DeserializeOwned, Error as _};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// Provider enums are open: known strings have typed constructors; future
// values remain exact strings, so upstream additions do not break decoding.
// A blank wire value is invalid except for order_class, whose published
// vocabulary includes "" as a legacy spelling of "simple".
pub trait OrderWire {
    fn wire(&self) -> &'static str;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenOrderEnum<E> {
    Known(E),
    Other(String),
}

impl<E: OrderWire> OpenOrderEnum<E> {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Known(value) => value.wire(),
            Self::Other(value) => value,
        }
    }
}

impl<E: Serialize> Serialize for OpenOrderEnum<E> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Known(value) => value.serialize(serializer),
            Self::Other(value) => serializer.serialize_str(value),
        }
    }
}

impl<'de, E: DeserializeOwned> Deserialize<'de> for OpenOrderEnum<E> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = String::deserialize(deserializer)?;
        // An empty string is accepted only if the known enum recognizes it.
        // This is true for legacy order_class="", not for side/status/etc.
        match serde_json::from_value(serde_json::Value::String(wire.clone())) {
            Ok(value) => Ok(Self::Known(value)),
            Err(_) if wire.is_empty() => Err(D::Error::custom("empty order enum value")),
            Err(_) => Ok(Self::Other(wire)),
        }
    }
}

pub type OrderSide = OpenOrderEnum<KnownOrderSide>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnownOrderSide {
    #[serde(rename = "buy")]
    Buy,
    #[serde(rename = "sell")]
    Sell,
}

impl OrderWire for KnownOrderSide {
    fn wire(&self) -> &'static str {
        match self {
            Self::Buy => "buy",
            Self::Sell => "sell",
        }
    }
}

pub type OrderState = OpenOrderEnum<KnownOrderState>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnownOrderState {
    #[serde(rename = "new")]
    New,
    #[serde(rename = "partially_filled")]
    PartiallyFilled,
    #[serde(rename = "filled")]
    Filled,
    #[serde(rename = "done_for_day")]
    DoneForDay,
    #[serde(rename = "canceled")]
    Canceled,
    #[serde(rename = "expired")]
    Expired,
    #[serde(rename = "replaced")]
    Replaced,
    #[serde(rename = "pending_cancel")]
    PendingCancel,
    #[serde(rename = "pending_replace")]
    PendingReplace,
    #[serde(rename = "accepted")]
    Accepted,
    #[serde(rename = "pending_new")]
    PendingNew,
    #[serde(rename = "accepted_for_bidding")]
    AcceptedForBidding,
    #[serde(rename = "stopped")]
    Stopped,
    #[serde(rename = "rejected")]
    Rejected,
    #[serde(rename = "suspended")]
    Suspended,
    #[serde(rename = "calculated")]
    Calculated,
    #[serde(rename = "held")]
    Held,
}

impl OrderWire for KnownOrderState {
    fn wire(&self) -> &'static str {
        match self {
            Self::New => "new",
            Self::PartiallyFilled => "partially_filled",
            Self::Filled => "filled",
            Self::DoneForDay => "done_for_day",
            Self::Canceled => "canceled",
            Self::Expired => "expired",
            Self::Replaced => "replaced",
            Self::PendingCancel => "pending_cancel",
            Self::PendingReplace => "pending_replace",
            Self::Accepted => "accepted",
            Self::PendingNew => "pending_new",
            Self::AcceptedForBidding => "accepted_for_bidding",
            Self::Stopped => "stopped",
            Self::Rejected => "rejected",
            Self::Suspended => "suspended",
            Self::Calculated => "calculated",
            Self::Held => "held",
        }
    }
}

pub type OrderType = OpenOrderEnum<KnownOrderType>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnownOrderType {
    #[serde(rename = "market")]
    Market,
    #[serde(rename = "limit")]
    Limit,
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "stop_limit")]
    StopLimit,
    #[serde(rename = "trailing_stop")]
    TrailingStop,
}

impl OrderWire for KnownOrderType {
    fn wire(&self) -> &'static str {
        match self {
            Self::Market => "market",
            Self::Limit => "limit",
            Self::Stop => "stop",
            Self::StopLimit => "stop_limit",
            Self::TrailingStop => "trailing_stop",
        }
    }
}

pub type OrderClass = OpenOrderEnum<KnownOrderClass>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnownOrderClass {
    #[serde(rename = "simple", alias = "")]
    Simple,
    #[serde(rename = "bracket")]
    Bracket,
    #[serde(rename = "oco")]
    Oco,
    #[serde(rename = "oto")]
    Oto,
    #[serde(rename = "mleg")]
    Mleg,
}

impl OrderWire for KnownOrderClass {
    fn wire(&self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Bracket => "bracket",
            Self::Oco => "oco",
            Self::Oto => "oto",
            Self::Mleg => "mleg",
        }
    }
}

pub type OrderTimeInForce = OpenOrderEnum<KnownOrderTimeInForce>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnownOrderTimeInForce {
    #[serde(rename = "day")]
    Day,
    #[serde(rename = "gtc")]
    Gtc,
    #[serde(rename = "opg")]
    Opg,
    #[serde(rename = "cls")]
    Cls,
    #[serde(rename = "ioc")]
    Ioc,
    #[serde(rename = "fok")]
    Fok,
}

impl OrderWire for KnownOrderTimeInForce {
    fn wire(&self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Gtc => "gtc",
            Self::Opg => "opg",
            Self::Cls => "cls",
            Self::Ioc => "ioc",
            Self::Fok => "fok",
        }
    }
}

pub type OrderPositionIntent = OpenOrderEnum<KnownOrderPositionIntent>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnownOrderPositionIntent {
    #[serde(rename = "buy_to_open")]
    BuyToOpen,
    #[serde(rename = "buy_to_close")]
    BuyToClose,
    #[serde(rename = "sell_to_open")]
    SellToOpen,
    #[serde(rename = "sell_to_close")]
    SellToClose,
}

impl OrderWire for KnownOrderPositionIntent {
    fn wire(&self) -> &'static str {
        match self {
            Self::BuyToOpen => "buy_to_open",
            Self::BuyToClose => "buy_to_close",
            Self::SellToOpen => "sell_to_open",
            Self::SellToClose => "sell_to_close",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::fmt::Debug;

    fn assert_known_values<E>(cases: &[(&str, E)])
    where
        E: Copy + Debug + PartialEq + Serialize + DeserializeOwned + OrderWire,
    {
        for (wire, known) in cases {
            let value = Value::String((*wire).into());
            let actual: OpenOrderEnum<E> = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(actual, OpenOrderEnum::Known(*known));
            assert_eq!(actual.as_str(), *wire);
            assert_eq!(serde_json::to_value(actual).unwrap(), value);
        }
    }

    fn assert_future_and_invalid_types<E>()
    where
        E: Debug + PartialEq + Serialize + DeserializeOwned + OrderWire,
    {
        let future: OpenOrderEnum<E> = serde_json::from_str(r#""FUTURE_ENUM""#).unwrap();
        assert_eq!(future, OpenOrderEnum::Other("FUTURE_ENUM".into()));
        assert_eq!(serde_json::to_string(&future).unwrap(), r#""FUTURE_ENUM""#);
        for invalid in ["null", "true", "42", "{}", "[]"] {
            assert!(serde_json::from_str::<OpenOrderEnum<E>>(invalid).is_err());
        }
    }

    #[test]
    fn all_documented_enum_members_are_typed_and_roundtrip_without_loss() {
        assert_known_values(&[("buy", KnownOrderSide::Buy), ("sell", KnownOrderSide::Sell)]);
        assert_known_values(&[
            ("new", KnownOrderState::New),
            ("partially_filled", KnownOrderState::PartiallyFilled),
            ("filled", KnownOrderState::Filled),
            ("done_for_day", KnownOrderState::DoneForDay),
            ("canceled", KnownOrderState::Canceled),
            ("expired", KnownOrderState::Expired),
            ("replaced", KnownOrderState::Replaced),
            ("pending_cancel", KnownOrderState::PendingCancel),
            ("pending_replace", KnownOrderState::PendingReplace),
            ("accepted", KnownOrderState::Accepted),
            ("pending_new", KnownOrderState::PendingNew),
            ("accepted_for_bidding", KnownOrderState::AcceptedForBidding),
            ("stopped", KnownOrderState::Stopped),
            ("rejected", KnownOrderState::Rejected),
            ("suspended", KnownOrderState::Suspended),
            ("calculated", KnownOrderState::Calculated),
            ("held", KnownOrderState::Held),
        ]);
        assert_known_values(&[
            ("market", KnownOrderType::Market),
            ("limit", KnownOrderType::Limit),
            ("stop", KnownOrderType::Stop),
            ("stop_limit", KnownOrderType::StopLimit),
            ("trailing_stop", KnownOrderType::TrailingStop),
        ]);
        assert_known_values(&[
            ("simple", KnownOrderClass::Simple),
            ("bracket", KnownOrderClass::Bracket),
            ("oco", KnownOrderClass::Oco),
            ("oto", KnownOrderClass::Oto),
            ("mleg", KnownOrderClass::Mleg),
        ]);
        assert_known_values(&[
            ("day", KnownOrderTimeInForce::Day),
            ("gtc", KnownOrderTimeInForce::Gtc),
            ("opg", KnownOrderTimeInForce::Opg),
            ("cls", KnownOrderTimeInForce::Cls),
            ("ioc", KnownOrderTimeInForce::Ioc),
            ("fok", KnownOrderTimeInForce::Fok),
        ]);
        assert_known_values(&[
            ("buy_to_open", KnownOrderPositionIntent::BuyToOpen),
            ("buy_to_close", KnownOrderPositionIntent::BuyToClose),
            ("sell_to_open", KnownOrderPositionIntent::SellToOpen),
            ("sell_to_close", KnownOrderPositionIntent::SellToClose),
        ]);
    }

    #[test]
    fn future_enum_members_survive_and_invalid_json_types_are_rejected() {
        assert_future_and_invalid_types::<KnownOrderSide>();
        assert_future_and_invalid_types::<KnownOrderState>();
        assert_future_and_invalid_types::<KnownOrderType>();
        assert_future_and_invalid_types::<KnownOrderClass>();
        assert_future_and_invalid_types::<KnownOrderTimeInForce>();
        assert_future_and_invalid_types::<KnownOrderPositionIntent>();

        assert!(serde_json::from_str::<OrderSide>(r#""""#).is_err());
        assert!(serde_json::from_str::<OrderState>(r#""""#).is_err());
        assert!(serde_json::from_str::<OrderType>(r#""""#).is_err());
        assert!(serde_json::from_str::<OrderTimeInForce>(r#""""#).is_err());
        assert!(serde_json::from_str::<OrderPositionIntent>(r#""""#).is_err());
    }

    #[test]
    fn deprecated_empty_order_class_is_a_known_simple_class() {
        let empty: OrderClass = serde_json::from_str(r#""""#).unwrap();
        assert_eq!(empty, OrderClass::Known(KnownOrderClass::Simple));
        assert_eq!(empty.as_str(), "simple");
    }
}
