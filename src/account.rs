use serde::Deserialize;

use crate::account_details::AccountDetails;
use crate::account_status::AccountStatus;

// Numeric monetary fields are JSON strings. Preserve their exact decimal spelling.
// The ID is validated but excluded from human-readable output.
#[derive(Deserialize)]
pub struct Account {
    #[serde(rename = "id")]
    _id: String,
    account_number: Option<String>,
    pub(crate) status: AccountStatus,
    currency: Option<String>,
    cash: Option<String>,
    equity: Option<String>,
    buying_power: Option<String>,
    pub(crate) trading_blocked: Option<bool>,
    pub(crate) crypto_status: Option<AccountStatus>,
    #[serde(flatten)]
    // The full typed response is for API fidelity today; downstream
    // account diagnostics and alternate renderers will consume it later.
    #[allow(dead_code)]
    pub details: AccountDetails,
}

pub fn decode_account(body: &[u8]) -> Result<Account, serde_json::Error> {
    serde_json::from_slice(body)
}

pub fn render_account_summary(account: &Account) -> String {
    format!(
        "Medea — Alpaca paper account\n\
         Account number: {}\n\
         Status: {}\n\
         Currency: {}\n\
         Cash: {}\n\
         Equity: {}\n\
         Buying power: {}\n\
         Trading blocked: {}\n\
         Crypto status: {}\n",
        account.account_number.as_deref().unwrap_or("not provided"),
        account.status,
        account.currency.as_deref().unwrap_or("not provided"),
        account.cash.as_deref().unwrap_or("not provided"),
        account.equity.as_deref().unwrap_or("not provided"),
        account.buying_power.as_deref().unwrap_or("not provided"),
        match account.trading_blocked {
            Some(true) => "yes",
            Some(false) => "no",
            None => "not provided",
        },
        account
            .crypto_status
            .as_ref()
            .map_or("not provided", AccountStatus::as_str),
    )
}

#[cfg(test)]
mod tests {
    use super::{decode_account, render_account_summary};
    use crate::account_details::OptionsLevel;
    use crate::account_status::AccountStatus;

    const RESPONSE: &str = r#"{
        "id":"private-id",
        "account_number":"PAPER-001",
        "status":"ACTIVE",
        "currency":"USD",
        "cash":"-12.345678901234567890",
        "equity":"1250.50",
        "buying_power":"2000",
        "trading_blocked":false,
        "crypto_status":"ACTIVE",
        "new_api_field":{"arbitrary":true}
    }"#;

    #[test]
    fn decodes_account_with_exact_decimal_strings() {
        let account = decode_account(RESPONSE.as_bytes()).unwrap();
        assert_eq!(account.cash.as_deref(), Some("-12.345678901234567890"));
        assert_eq!(account.equity.as_deref(), Some("1250.50"));
        assert_eq!(account.buying_power.as_deref(), Some("2000"));
        assert_eq!(account.status, AccountStatus::Active);
        assert_eq!(account.crypto_status, Some(AccountStatus::Active));
    }

    #[test]
    fn renders_readable_summary_without_internal_id() {
        let account = decode_account(RESPONSE.as_bytes()).unwrap();
        let summary = render_account_summary(&account);
        assert!(summary.starts_with("Medea — Alpaca paper account\n"));
        assert!(summary.contains("Account number: PAPER-001\n"));
        assert!(summary.contains("Cash: -12.345678901234567890\n"));
        assert!(summary.contains("Trading blocked: no\n"));
        assert!(summary.contains("Crypto status: ACTIVE\n"));
        assert!(!summary.contains("private-id"));
    }

    #[test]
    fn missing_optional_crypto_status_is_allowed() {
        let response = RESPONSE.replace("        \"crypto_status\":\"ACTIVE\",\n", "");
        let account = decode_account(response.as_bytes()).unwrap();
        assert!(account.crypto_status.is_none());
        assert!(render_account_summary(&account).contains("Crypto status: not provided\n"));
    }

    #[test]
    fn account_and_crypto_status_are_independent_enums() {
        let response = RESPONSE
            .replace(r#""status":"ACTIVE""#, r#""status":"LIMITED""#)
            .replace(
                r#""crypto_status":"ACTIVE""#,
                r#""crypto_status":"PAPER_ONLY""#,
            );
        let account = decode_account(response.as_bytes()).unwrap();
        assert_eq!(account.status, AccountStatus::Limited);
        assert_eq!(account.crypto_status, Some(AccountStatus::PaperOnly));
        let summary = render_account_summary(&account);
        assert!(summary.contains("Status: LIMITED\n"));
        assert!(summary.contains("Crypto status: PAPER_ONLY\n"));
    }

    #[test]
    fn unknown_future_statuses_are_retained_in_both_fields() {
        let response = RESPONSE
            .replace(r#""status":"ACTIVE""#, r#""status":"NEW_ACCOUNT_STATUS""#)
            .replace(
                r#""crypto_status":"ACTIVE""#,
                r#""crypto_status":"NEW_CRYPTO_STATUS""#,
            );
        let account = decode_account(response.as_bytes()).unwrap();
        assert_eq!(
            account.status,
            AccountStatus::Other("NEW_ACCOUNT_STATUS".into())
        );
        assert_eq!(
            account.crypto_status,
            Some(AccountStatus::Other("NEW_CRYPTO_STATUS".into()))
        );
        let summary = render_account_summary(&account);
        assert!(summary.contains("Status: NEW_ACCOUNT_STATUS\n"));
        assert!(summary.contains("Crypto status: NEW_CRYPTO_STATUS\n"));
    }

    #[test]
    fn null_optional_crypto_status_is_allowed() {
        let response = RESPONSE.replace(r#""crypto_status":"ACTIVE""#, r#""crypto_status":null"#);
        let account = decode_account(response.as_bytes()).unwrap();
        assert_eq!(account.crypto_status, None);
        assert!(render_account_summary(&account).contains("Crypto status: not provided\n"));
    }

    #[test]
    fn required_status_cannot_be_missing_or_null() {
        let null = RESPONSE.replace(r#""status":"ACTIVE""#, r#""status":null"#);
        let missing = RESPONSE.replace(r#""status":"ACTIVE","#, "");
        assert!(decode_account(null.as_bytes()).is_err());
        assert!(decode_account(missing.as_bytes()).is_err());
    }

    #[test]
    fn status_fields_must_be_json_strings_when_present() {
        let invalid_status = RESPONSE.replace(r#""status":"ACTIVE""#, r#""status":1"#);
        let invalid_crypto =
            RESPONSE.replace(r#""crypto_status":"ACTIVE""#, r#""crypto_status":false"#);
        assert!(decode_account(invalid_status.as_bytes()).is_err());
        assert!(decode_account(invalid_crypto.as_bytes()).is_err());
    }

    #[test]
    fn blocked_trading_uses_yes() {
        let response = RESPONSE.replace(r#""trading_blocked":false"#, r#""trading_blocked":true"#);
        let account = decode_account(response.as_bytes()).unwrap();
        assert!(render_account_summary(&account).contains("Trading blocked: yes\n"));
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(decode_account(b"{oops").is_err());
    }

    #[test]
    fn rejects_missing_required_fields() {
        assert!(decode_account(br#"{"id":"private-id"}"#).is_err());
    }

    #[test]
    fn rejects_numeric_money_values() {
        let response = RESPONSE.replace(r#""cash":"-12.345678901234567890""#, r#""cash":12.34"#);
        assert!(decode_account(response.as_bytes()).is_err());
    }
    const FULL_SCHEMA_RESPONSE: &str = r#"{
        "id":"example-id",
        "status":"ACTIVE",
        "account_number":"PAPER-001",
        "crypto_status":"PAPER_ONLY",
        "currency":"USD",
        "cash":"-12.345678901234567890123456789",
        "equity":"1000.00000000000000000001",
        "buying_power":"2e0",
        "trading_blocked":false,
        "account_blocked":true,
        "accrued_fees":"0.00300",
        "balance_asof":"2026-11-12",
        "created_at":"2026-11-11T23:59:59.123456789Z",
        "initial_margin":"-0.23",
        "intraday_adjustments":"100",
        "last_equity":"1001",
        "last_maintenance_margin":"12.30",
        "long_market_value":"800.55",
        "maintenance_margin":"12.31",
        "multiplier":"4",
        "non_marginable_buying_power":"15.35",
        "options_approved_level":3,
        "options_buying_power":"45.50",
        "options_trading_level":2,
        "pending_reg_taf_fees":"0.00005",
        "pending_transfer_in":"0",
        "pending_transfer_out":"100.10",
        "portfolio_value":"1000.00000000000000000001",
        "regt_buying_power":"200.125",
        "short_market_value":"-3.45",
        "shorting_enabled":true,
        "sma":"10.00",
        "trade_suspended_by_user":false,
        "transfers_blocked":true,
        "future_new_property":{"pi":"3.1415926535897932384626"}
    }"#;

    #[test]
    fn decodes_every_published_account_schema_field_with_correct_types() {
        let account = decode_account(FULL_SCHEMA_RESPONSE.as_bytes()).unwrap();
        assert_eq!(account.account_number.as_deref(), Some("PAPER-001"));
        assert_eq!(account.status, AccountStatus::Active);
        assert_eq!(account.crypto_status, Some(AccountStatus::PaperOnly));
        assert_eq!(account.currency.as_deref(), Some("USD"));
        assert_eq!(
            account.cash.as_deref(),
            Some("-12.345678901234567890123456789")
        );
        assert_eq!(account.equity.as_deref(), Some("1000.00000000000000000001"));
        assert_eq!(account.buying_power.as_deref(), Some("2e0"));
        assert_eq!(account.trading_blocked, Some(false));
        let details = &account.details;
        assert_eq!(details.account_blocked, Some(true));
        assert_eq!(details.accrued_fees.as_deref(), Some("0.00300"));
        assert_eq!(details.balance_asof.as_deref(), Some("2026-11-12"));
        assert_eq!(
            details.created_at.as_deref(),
            Some("2026-11-11T23:59:59.123456789Z")
        );
        assert_eq!(details.initial_margin.as_deref(), Some("-0.23"));
        assert_eq!(details.intraday_adjustments.as_deref(), Some("100"));
        assert_eq!(details.last_equity.as_deref(), Some("1001"));
        assert_eq!(details.last_maintenance_margin.as_deref(), Some("12.30"));
        assert_eq!(details.long_market_value.as_deref(), Some("800.55"));
        assert_eq!(details.maintenance_margin.as_deref(), Some("12.31"));
        assert_eq!(details.multiplier.as_deref(), Some("4"));
        assert_eq!(
            details.non_marginable_buying_power.as_deref(),
            Some("15.35")
        );
        assert_eq!(details.options_approved_level, Some(OptionsLevel::Spreads));
        assert_eq!(details.options_buying_power.as_deref(), Some("45.50"));
        assert_eq!(details.options_trading_level, Some(OptionsLevel::Long));
        assert_eq!(details.pending_reg_taf_fees.as_deref(), Some("0.00005"));
        assert_eq!(details.pending_transfer_in.as_deref(), Some("0"));
        assert_eq!(details.pending_transfer_out.as_deref(), Some("100.10"));
        assert_eq!(
            details.portfolio_value.as_deref(),
            Some("1000.00000000000000000001")
        );
        assert_eq!(details.regt_buying_power.as_deref(), Some("200.125"));
        assert_eq!(details.short_market_value.as_deref(), Some("-3.45"));
        assert_eq!(details.shorting_enabled, Some(true));
        assert_eq!(details.sma.as_deref(), Some("10.00"));
        assert_eq!(details.trade_suspended_by_user, Some(false));
        assert_eq!(details.transfers_blocked, Some(true));
    }

    #[test]
    fn account_requires_only_id_and_status_and_supports_null_account_number() {
        for input in [
            r#"{"id":"example-id","status":"ACTIVE"}"#,
            r#"{"id":"example-id","status":"ACTIVE","account_number":null}"#,
        ] {
            let account = decode_account(input.as_bytes()).unwrap();
            assert_eq!(account.account_number, None);
            assert_eq!(account.currency, None);
            assert_eq!(account.cash, None);
            assert_eq!(account.equity, None);
            assert_eq!(account.buying_power, None);
            assert_eq!(account.trading_blocked, None);
            assert_eq!(account.crypto_status, None);
            assert_eq!(account.details.options_trading_level, None);
        }
    }

    #[test]
    fn sparse_account_summary_explicitly_marks_unavailable_fields() {
        let account =
            decode_account(br#"{"id":"private-id","status":"ONBOARDING","account_number":null}"#)
                .unwrap();
        let summary = render_account_summary(&account);
        for line in [
            "Status: ONBOARDING\n",
            "Account number: not provided\n",
            "Currency: not provided\n",
            "Cash: not provided\n",
            "Equity: not provided\n",
            "Buying power: not provided\n",
            "Trading blocked: not provided\n",
            "Crypto status: not provided\n",
        ] {
            assert!(
                summary.contains(line),
                "Missing expected summary line: {line}"
            );
        }
        assert!(!summary.contains("private-id"));
    }

    #[test]
    fn optional_null_and_missing_are_distinct_from_false_and_zero() {
        let account = decode_account(
            br#"{"id":"id","status":"ACTIVE","trading_blocked":false,"transfers_blocked":null,"options_approved_level":0,"cash":"0"}"#,
        )
        .unwrap();
        assert_eq!(account.trading_blocked, Some(false));
        assert_eq!(account.cash.as_deref(), Some("0"));
        assert_eq!(account.equity, None);
        assert_eq!(account.details.transfers_blocked, None);
        assert_eq!(
            account.details.options_approved_level,
            Some(OptionsLevel::Disabled)
        );
    }

    #[test]
    fn unknown_future_options_level_is_preserved_while_raw_json_stays_untouched() {
        let input =
            br#"{"id":"id","status":"ACTIVE","options_trading_level":8,"future_new_field":{"x":"3.1400"}}"#;
        let account = decode_account(input).unwrap();
        assert_eq!(
            account.details.options_trading_level,
            Some(OptionsLevel::Other(8))
        );
        let parsed: serde_json::Value = serde_json::from_slice(input).unwrap();
        assert_eq!(parsed["future_new_field"]["x"], "3.1400");
        // The CLI's --out path receives original input bytes, not reserialization.
        assert!(input.windows(b"3.1400".len()).any(|w| w == b"3.1400"));
    }

    #[test]
    fn rejects_wrong_types_for_every_optional_field_kind() {
        for (name, wrong_value) in [
            ("account_number", "42"),
            ("currency", "false"),
            ("cash", "12.34"),
            ("equity", "1"),
            ("buying_power", "0"),
            ("trading_blocked", r#""true""#),
            ("crypto_status", "false"),
            ("account_blocked", "0"),
            ("accrued_fees", "0.5"),
            ("created_at", "123"),
            ("initial_margin", "1.2"),
            ("options_approved_level", "1.5"),
            ("options_trading_level", r#""3""#),
            ("shorting_enabled", r#""true""#),
            ("transfers_blocked", "1"),
        ] {
            let response = format!(r#"{{"id":"id","status":"ACTIVE","{name}":{wrong_value}}}"#);
            assert!(
                decode_account(response.as_bytes()).is_err(),
                "{name} should reject {wrong_value}"
            );
        }
    }

    #[test]
    fn rejects_non_objects_and_missing_required_account_fields() {
        for response in [
            "[]",
            "null",
            "{}",
            r#"{"id":"example-id"}"#,
            r#"{"status":"ACTIVE"}"#,
            r#"{"id":null,"status":"ACTIVE"}"#,
            r#"{"id":"id","status":null}"#,
        ] {
            assert!(decode_account(response.as_bytes()).is_err(), "{response}");
        }
    }
}
