use serde::Deserialize;

// Numeric monetary fields are JSON strings. Preserve their exact decimal spelling.
// The ID is validated but excluded from human-readable output.
#[derive(Deserialize)]
pub struct Account {
    #[serde(rename = "id")]
    _id: String,
    account_number: String,
    status: String,
    currency: String,
    cash: String,
    equity: String,
    buying_power: String,
    trading_blocked: bool,
    crypto_status: Option<String>,
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
        account.crypto_status.as_deref().unwrap_or("not provided"),
    )
}

#[cfg(test)]
mod tests {
    use super::{decode_account, render_account_summary};

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
        assert_eq!(account.crypto_status.as_deref(), Some("ACTIVE"));
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
