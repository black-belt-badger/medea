# GET /v2/account — M3 response coverage

Scope: [Medea #33](https://github.com/black-belt-badger/medea/issues/33), paired with [Kirke #32](https://github.com/black-belt-badger/kirke/issues/32) and [Kirke PR #46](https://github.com/black-belt-badger/kirke/pull/46).

Sources verified 2026-11-13: [Alpaca GET /v2/account](https://docs.alpaca.markets/us/reference/getaccount-1) and the [machine-readable OpenAPI schema](https://docs.alpaca.markets/us/reference/getaccount-1.md).

**Coverage: all 34 fields in the published Account schema.** This phase completes typed response-property coverage, following M3's status-enum phase. Shared terminal color, diagnostics and Unix output behavior are a separate, later M3 decision.

## Request contract

| Alpaca contract | Medea implementation |
| --- | --- |
| `GET https://paper-api.alpaca.markets/v2/account` | Fixed authenticated HTTPS paper-only GET; no live trading |
| Query/path/body parameters | **None**; never invent REST filters |
| Success response | One JSON account object; typed decoding for all known fields |
| Non-2xx and transport errors | Sanitized status/error messages; no provider response body or secrets |
| File output | `medea account --out PATH` writes the **original complete JSON bytes**, owner-only, atomically |

## Response-property coverage

The current OpenAPI `required` list is only **`id` and `status`**. Notably, `account_number` is nullable. Other documented response fields may be missing (and the implementation tolerates explicit null for optional fields). Missing and null both become `None` in the typed model; **the untouched raw JSON export** retains the original distinction. Present `false` and the monetary string `"0"` remain distinct from `None`.

| Property | Rust type | Requirement | Model |
| --- | --- | --- | --- |
| `account_blocked` | `Option<bool>` | Optional | `AccountDetails` |
| `account_number` | `Option<String> (nullable)` | Optional | `Account` |
| `accrued_fees` | `Option<String>` | Optional | `AccountDetails` |
| `balance_asof` | `Option<String>` | Optional | `AccountDetails` |
| `buying_power` | `Option<String>` | Optional | `Account` |
| `cash` | `Option<String>` | Optional | `Account` |
| `created_at` | `Option<String>` | Optional | `AccountDetails` |
| `crypto_status` | `Option<AccountStatus>` | Optional | `Account` |
| `currency` | `Option<String>` | Optional | `Account` |
| `equity` | `Option<String>` | Optional | `Account` |
| `id` | `String` | Required | `Account` |
| `initial_margin` | `Option<String>` | Optional | `AccountDetails` |
| `intraday_adjustments` | `Option<String>` | Optional | `AccountDetails` |
| `last_equity` | `Option<String>` | Optional | `AccountDetails` |
| `last_maintenance_margin` | `Option<String>` | Optional | `AccountDetails` |
| `long_market_value` | `Option<String>` | Optional | `AccountDetails` |
| `maintenance_margin` | `Option<String>` | Optional | `AccountDetails` |
| `multiplier` | `Option<String>` | Optional | `AccountDetails` |
| `non_marginable_buying_power` | `Option<String>` | Optional | `AccountDetails` |
| `options_approved_level` | `Option<OptionsLevel>` | Optional | `AccountDetails` |
| `options_buying_power` | `Option<String>` | Optional | `AccountDetails` |
| `options_trading_level` | `Option<OptionsLevel>` | Optional | `AccountDetails` |
| `pending_reg_taf_fees` | `Option<String>` | Optional | `AccountDetails` |
| `pending_transfer_in` | `Option<String>` | Optional | `AccountDetails` |
| `pending_transfer_out` | `Option<String>` | Optional | `AccountDetails` |
| `portfolio_value` | `Option<String>` | Optional | `AccountDetails` |
| `regt_buying_power` | `Option<String>` | Optional | `AccountDetails` |
| `short_market_value` | `Option<String>` | Optional | `AccountDetails` |
| `shorting_enabled` | `Option<bool>` | Optional | `AccountDetails` |
| `sma` | `Option<String>` | Optional | `AccountDetails` |
| `status` | `AccountStatus` | Required | `Account` |
| `trade_suspended_by_user` | `Option<bool>` | Optional | `AccountDetails` |
| `trading_blocked` | `Option<bool>` | Optional | `Account` |
| `transfers_blocked` | `Option<bool>` | Optional | `AccountDetails` |

## Validation and forward compatibility

- `status` and optional `crypto_status` share `AccountStatus`: all 14 documented values are named variants, and `Other(String)` retains any future status spelling.
- `options_approved_level` and `options_trading_level` are integer enums: `0` disabled, `1` covered call/cash-secured put, `2` long call/put, `3` spreads/straddles. `OptionsLevel::Other(i64)` preserves unrecognized integer levels. JSON fractions, quoted strings and booleans are rejected.
- Monetary/buying-power/margin values remain exact **JSON strings**, held as `String`; no floating-point conversion or reformatting. `balance_asof` and `created_at` retain their original text, including fractional-second precision.
- Unknown upstream properties (for example `effective_buying_power`, `position_market_value`, and configuration objects found in example responses) do not break typed decoding and remain present in the complete raw JSON export.
- Older/deprecated properties omitted by Alpaca are never required.

## CLI behavior and tests

The visible account summary is intentionally unchanged for fully populated responses. When a field is missing or null, the summary prints **`not provided`**, not a guessed zero or `false`. It continues to omit the account UUID.

Offline tests cover full-schema fixtures; all 34 property types; missing/null/present-zero/present-false; exact long-decimal values; option enum known and future cases; invalid JSON types; unknown extra fields; and the existing status, request and private-file behavior. Live paper-account integration is opt-in and never belongs in public CI.

## Not part of this implementation

This typed-model phase does **not** add strict UUID/date-format string parsing, new CLI options, terminal color, diagnostics, financial explanations or different Unix stream/output modes. Those should follow the planned cross-cutting contract in [Medea #44](https://github.com/black-belt-badger/medea/issues/44) and [Kirke #44](https://github.com/black-belt-badger/kirke/issues/44), after all M3 endpoint audits. Keep the parent account issue open until final acceptance review.
