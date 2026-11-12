use std::fmt;

// Deliberately do not derive Debug: secrets must never appear in error output.
pub struct Credentials {
    pub key_id: String,
    pub secret_key: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CredentialError {
    MissingEnvironmentVariable(&'static str),
    EmptyEnvironmentVariable(&'static str),
}

impl fmt::Display for CredentialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEnvironmentVariable(name) => {
                write!(f, "Missing environment variable: {name}")
            }
            Self::EmptyEnvironmentVariable(name) => {
                write!(f, "Empty environment variable: {name}")
            }
        }
    }
}

pub fn load_credentials() -> Result<Credentials, CredentialError> {
    validate_credentials(
        std::env::var("APCA_API_KEY_ID").ok().as_deref(),
        std::env::var("APCA_API_SECRET_KEY").ok().as_deref(),
    )
}

pub fn validate_credentials(
    key: Option<&str>,
    secret: Option<&str>,
) -> Result<Credentials, CredentialError> {
    fn require(name: &'static str, value: Option<&str>) -> Result<String, CredentialError> {
        match value {
            None => Err(CredentialError::MissingEnvironmentVariable(name)),
            Some(value) if value.trim().is_empty() => {
                Err(CredentialError::EmptyEnvironmentVariable(name))
            }
            Some(value) => Ok(value.to_owned()),
        }
    }

    Ok(Credentials {
        key_id: require("APCA_API_KEY_ID", key)?,
        secret_key: require("APCA_API_SECRET_KEY", secret)?,
    })
}

#[cfg(test)]
mod tests {
    use super::{CredentialError, validate_credentials};

    #[test]
    fn accepts_credentials() {
        let creds = validate_credentials(Some("example-key"), Some("example-secret")).unwrap();
        assert_eq!(creds.key_id, "example-key");
        assert_eq!(creds.secret_key, "example-secret");
    }

    #[test]
    fn rejects_missing_key() {
        assert!(matches!(
            validate_credentials(None, Some("secret")),
            Err(CredentialError::MissingEnvironmentVariable(
                "APCA_API_KEY_ID"
            ))
        ));
    }

    #[test]
    fn rejects_missing_secret() {
        assert!(matches!(
            validate_credentials(Some("key"), None),
            Err(CredentialError::MissingEnvironmentVariable(
                "APCA_API_SECRET_KEY"
            ))
        ));
    }

    #[test]
    fn rejects_empty_key() {
        assert!(matches!(
            validate_credentials(Some(""), Some("secret")),
            Err(CredentialError::EmptyEnvironmentVariable("APCA_API_KEY_ID"))
        ));
    }

    #[test]
    fn rejects_whitespace_only_secret() {
        assert!(matches!(
            validate_credentials(Some("key"), Some(" \t")),
            Err(CredentialError::EmptyEnvironmentVariable(
                "APCA_API_SECRET_KEY"
            ))
        ));
    }

    #[test]
    fn error_messages_do_not_include_secrets() {
        let err = validate_credentials(Some(""), Some("example-secret"))
            .err()
            .unwrap();
        assert_eq!(
            err.to_string(),
            "Empty environment variable: APCA_API_KEY_ID"
        );
        assert!(!err.to_string().contains("example-secret"));
    }
}
