use serde::Deserialize;

use crate::account_status::AccountStatus;

// Numeric monetary fields are JSON strings. Preserve their exact decimal spelling.
// The ID is validated but excluded from human-readable output.
#[derive(Deserialize)]
pub struct Account {
    #[serde(rename = "id")]
    _id: String,
    account_number: String,
    status: AccountStatus,
    currency: String,
    cash: String,
    equity: String,
    buying_power: String,
    trading_blocked: bool,
    crypto_status: Option<AccountStatus>,
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
        account.account_number,
        account.status,
        account.currency,
        account.cash,
        account.equity,
        account.buying_power,
        if account.trading_blocked { "yes" } else { "no" },
        account
            .crypto_status
            .as_ref()
            .map_or("not provided", AccountStatus::as_str),
    )
}

#[cfg(test)]
mod tests {
    use super::{decode_account, render_account_summary};
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
        assert_eq!(account.cash, "-12.345678901234567890");
        assert_eq!(account.equity, "1250.50");
        assert_eq!(account.buying_power, "2000");
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
}
