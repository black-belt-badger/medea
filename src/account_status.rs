use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Known Alpaca Trading API statuses, shared by `status` and `crypto_status`.
///
/// Future wire values are represented by `Other` rather than causing an
/// otherwise valid account response to fail decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountStatus {
    Inactive,
    PaperOnly,
    Onboarding,
    Submitted,
    SubmissionFailed,
    ActionRequired,
    AccountUpdated,
    ApprovalPending,
    Approved,
    Rejected,
    Active,
    AccountClosed,
    AccountClosedPending,
    Limited,
    Other(String),
}

impl AccountStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Inactive => "INACTIVE",
            Self::PaperOnly => "PAPER_ONLY",
            Self::Onboarding => "ONBOARDING",
            Self::Submitted => "SUBMITTED",
            Self::SubmissionFailed => "SUBMISSION_FAILED",
            Self::ActionRequired => "ACTION_REQUIRED",
            Self::AccountUpdated => "ACCOUNT_UPDATED",
            Self::ApprovalPending => "APPROVAL_PENDING",
            Self::Approved => "APPROVED",
            Self::Rejected => "REJECTED",
            Self::Active => "ACTIVE",
            Self::AccountClosed => "ACCOUNT_CLOSED",
            Self::AccountClosedPending => "ACCOUNT_CLOSED_PENDING",
            Self::Limited => "LIMITED",
            Self::Other(value) => value.as_str(),
        }
    }
}

impl From<String> for AccountStatus {
    fn from(value: String) -> Self {
        match value.as_str() {
            "INACTIVE" => Self::Inactive,
            "PAPER_ONLY" => Self::PaperOnly,
            "ONBOARDING" => Self::Onboarding,
            "SUBMITTED" => Self::Submitted,
            "SUBMISSION_FAILED" => Self::SubmissionFailed,
            "ACTION_REQUIRED" => Self::ActionRequired,
            "ACCOUNT_UPDATED" => Self::AccountUpdated,
            "APPROVAL_PENDING" => Self::ApprovalPending,
            "APPROVED" => Self::Approved,
            "REJECTED" => Self::Rejected,
            "ACTIVE" => Self::Active,
            "ACCOUNT_CLOSED" => Self::AccountClosed,
            "ACCOUNT_CLOSED_PENDING" => Self::AccountClosedPending,
            "LIMITED" => Self::Limited,
            _ => Self::Other(value),
        }
    }
}

impl fmt::Display for AccountStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for AccountStatus {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::from)
    }
}

impl Serialize for AccountStatus {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::AccountStatus;

    #[test]
    fn every_documented_status_decodes_and_round_trips_exactly() {
        let documented = [
            ("INACTIVE", AccountStatus::Inactive),
            ("PAPER_ONLY", AccountStatus::PaperOnly),
            ("ONBOARDING", AccountStatus::Onboarding),
            ("SUBMITTED", AccountStatus::Submitted),
            ("SUBMISSION_FAILED", AccountStatus::SubmissionFailed),
            ("ACTION_REQUIRED", AccountStatus::ActionRequired),
            ("ACCOUNT_UPDATED", AccountStatus::AccountUpdated),
            ("APPROVAL_PENDING", AccountStatus::ApprovalPending),
            ("APPROVED", AccountStatus::Approved),
            ("REJECTED", AccountStatus::Rejected),
            ("ACTIVE", AccountStatus::Active),
            ("ACCOUNT_CLOSED", AccountStatus::AccountClosed),
            (
                "ACCOUNT_CLOSED_PENDING",
                AccountStatus::AccountClosedPending,
            ),
            ("LIMITED", AccountStatus::Limited),
        ];

        for (wire, expected) in documented {
            let json = format!("{wire:?}");
            let decoded: AccountStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, expected);
            assert_eq!(decoded.as_str(), wire);
            assert_eq!(decoded.to_string(), wire);
            assert_eq!(serde_json::to_string(&decoded).unwrap(), json);
        }
    }

    #[test]
    fn unfamiliar_future_status_is_preserved_without_normalizing_it() {
        let status: AccountStatus = serde_json::from_str(r#""NEW_CRYPTO_STATUS""#).unwrap();
        assert_eq!(status, AccountStatus::Other("NEW_CRYPTO_STATUS".into()));
        assert_eq!(status.as_str(), "NEW_CRYPTO_STATUS");
        assert_eq!(
            serde_json::to_string(&status).unwrap(),
            r#""NEW_CRYPTO_STATUS""#
        );
    }

    #[test]
    fn status_must_be_a_json_string() {
        for invalid in ["null", "false", "42", "{}", "[]"] {
            assert!(serde_json::from_str::<AccountStatus>(invalid).is_err());
        }
    }
}
