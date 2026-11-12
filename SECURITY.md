# Security policy

Medea is an **unofficial** Rust CLI supporting read-only access to Alpaca **paper trading accounts**. It is not affiliated with Alpaca Markets.

## Supported versions

No tagged release has been published yet. We review and address security reports against the current `main` branch; older commits are not supported.

## Report vulnerabilities privately

**Do not disclose vulnerabilities, exploits, API credentials, account numbers or private account data in public issues or pull requests.**

Use [GitHub private vulnerability reporting](https://github.com/black-belt-badger/medea/security/advisories) and select **Report a vulnerability**. Provide a description, impact, reproduction steps, and a sanitized proof of concept. Do not include working API keys or full account JSON.

We intend to review responsible reports but cannot promise a fixed response or remediation deadline.

For ordinary bugs or feature requests, use the [issue templates](https://github.com/black-belt-badger/medea/issues/new/choose).

## Credentials and account data

Medea reads `APCA_API_KEY_ID` and `APCA_API_SECRET_KEY` from the environment and sends requests only to the paper Trading API. Keep credentials outside version control, restrict local file permissions, and redact outputs and logs. Account summaries and complete JSON responses may contain sensitive financial information.
