//! Pure account observations; do not infer trading eligibility from status.

use crate::account::Account;
use crate::account_status::AccountStatus;
use crate::diagnostics::{Finding, Severity};

#[allow(dead_code)]
pub fn account_findings(account: &Account) -> Vec<Finding> {
    let mut findings = Vec::new();
    if account.status != AccountStatus::Active {
        findings.push(Finding {
            code: "ACCOUNT_STATUS_NOT_ACTIVE",
            severity: Severity::Warning,
            path: "status",
            message: "Alpaca reports an account status other than ACTIVE.",
            explanation: Some(
                "Account status is a distinct provider field; do not infer complete trading permissions from it.",
            ),
        });
    }
    match account.trading_blocked {
        Some(true) => findings.push(Finding {
            code: "ACCOUNT_TRADING_BLOCKED",
            severity: Severity::Error,
            path: "trading_blocked",
            message: "Alpaca reports trading_blocked=true.",
            explanation: Some(
                "This restriction is independent of status; an ACTIVE status does not override it.",
            ),
        }),
        Some(false) => {}
        None => findings.push(Finding {
            code: "ACCOUNT_TRADING_BLOCKED_UNKNOWN",
            severity: Severity::Info,
            path: "trading_blocked",
            message: "Alpaca did not provide a trading_blocked value.",
            explanation: Some("Absence is not evidence of trading permission."),
        }),
    }
    match &account.crypto_status {
        Some(AccountStatus::Active) | None => {}
        Some(_) => findings.push(Finding {
            code: "ACCOUNT_CRYPTO_STATUS_NOT_ACTIVE",
            severity: Severity::Info,
            path: "crypto_status",
            message: "Alpaca reports a crypto_status other than ACTIVE.",
            explanation: Some(
                "This is a separate provider status; its value alone does not establish trading eligibility.",
            ),
        }),
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::account_findings;
    use crate::account::decode_account;
    use crate::diagnostics::Severity;

    fn codes(json: &str) -> Vec<(&'static str, Severity, &'static str)> {
        let account = decode_account(json.as_bytes()).unwrap();
        account_findings(&account)
            .into_iter()
            .map(|item| (item.code, item.severity, item.path))
            .collect()
    }

    #[test]
    fn severity_vocabulary_is_stable_and_lowercase() {
        assert_eq!(
            [Severity::Info, Severity::Warning, Severity::Error].map(Severity::as_str),
            ["info", "warning", "error"]
        );
    }

    #[test]
    fn active_and_explicitly_unblocked_is_not_a_trading_eligibility_claim() {
        assert_eq!(
            codes(
                r#"{"id":"secret","status":"ACTIVE","trading_blocked":false,"crypto_status":"ACTIVE"}"#
            ),
            []
        );
    }

    #[test]
    fn true_trading_blocked_is_an_error_even_with_active_status() {
        assert_eq!(
            codes(
                r#"{"id":"secret","status":"ACTIVE","trading_blocked":true,"crypto_status":"ACTIVE"}"#
            ),
            [(
                "ACCOUNT_TRADING_BLOCKED",
                Severity::Error,
                "trading_blocked"
            )]
        );
    }

    #[test]
    fn independent_provider_fields_have_stable_parity_codes() {
        assert_eq!(
            codes(r#"{"id":"secret","status":"LIMITED","crypto_status":"PAPER_ONLY"}"#),
            [
                ("ACCOUNT_STATUS_NOT_ACTIVE", Severity::Warning, "status"),
                (
                    "ACCOUNT_TRADING_BLOCKED_UNKNOWN",
                    Severity::Info,
                    "trading_blocked"
                ),
                (
                    "ACCOUNT_CRYPTO_STATUS_NOT_ACTIVE",
                    Severity::Info,
                    "crypto_status"
                ),
            ]
        );
    }

    #[test]
    fn absent_crypto_status_is_not_assumed_to_mean_restriction() {
        assert_eq!(
            codes(r#"{"id":"secret","status":"ACTIVE","trading_blocked":false}"#),
            []
        );
    }

    #[test]
    fn sensitive_identifiers_and_balance_values_never_appear_in_findings() {
        let payload = r#"{"id":"PRIVATE-ID-123","account_number":"PRIVATE-ACCOUNT","cash":"123456.789","status":"ACTIVE","trading_blocked":true}"#;
        let account = decode_account(payload.as_bytes()).unwrap();
        let rendered = format!("{:?}", account_findings(&account));
        assert!(rendered.contains("ACCOUNT_TRADING_BLOCKED"));
        for secret in ["PRIVATE-ID-123", "PRIVATE-ACCOUNT", "123456.789"] {
            assert!(!rendered.contains(secret));
        }
    }
}
