# M3 — account endpoint coverage and parity

Scope: [Medea #33](https://github.com/black-belt-badger/medea/issues/33) paired with [Kirke #32](https://github.com/black-belt-badger/kirke/issues/32), the first, narrow account-status enum improvement. The full M3 response audit remains open.

API reference: [Alpaca GET /v2/account](https://docs.alpaca.markets/us/reference/getaccount-1).

## Request coverage

| Alpaca contract | Medea implementation | Current state |
| --- | --- | --- |
| GET `/v2/account`, no path/query/body arguments | Fixed `GET` request without parameters or body | Covered, regression-tested |
| Trading host and credentials | HTTPS `paper-api.alpaca.markets`, paper-only credentials from `APCA_API_KEY_ID` and `APCA_API_SECRET_KEY` | Covered |
| One account object on HTTP success | Typed decoder and concise human summary | Covered for currently displayed fields |
| Non-2xx or connection failures | Sanitized status and transport errors; no provider response bodies | Covered, remaining negative cases later in M3 |
| Original, complete provider data | `account --out PATH` writes original JSON bytes atomically with owner-only permissions | Covered |
| CLI options | No REST parameters; optional `--out PATH` controls local export only | Covered |

## Response coverage

| Field group | Current representation | M3 follow-up |
| --- | --- | --- |
| `status` | Required, typed `AccountStatus` enum | Done |
| `crypto_status` | Optional, same `AccountStatus` enum; missing or explicit null = `None` | Done |
| `id`, `account_number`, `currency` | Required strings | Reassess schema nullability |
| `cash`, `equity`, `buying_power` | Required **exact JSON decimal strings**, never floats | Keep exact |
| `trading_blocked` | Required boolean | Done for summary |
| Margin, buying-power variations, option levels, previous balances, trading/suspension flags, timestamps, deprecated fields | Available in the original `--out` JSON; not all typed yet | **Full schema audit is future M3 work** |
| Newly added properties | Raw output preserves them unchanged | Done for export |

### Account-status vocabulary

The two response fields share these known Alpaca statuses:

`INACTIVE`, `PAPER_ONLY`, `ONBOARDING`, `SUBMITTED`, `SUBMISSION_FAILED`, `ACTION_REQUIRED`, `ACCOUNT_UPDATED`, `APPROVAL_PENDING`, `APPROVED`, `REJECTED`, `ACTIVE`, `ACCOUNT_CLOSED`, `ACCOUNT_CLOSED_PENDING`, `LIMITED`.

Each known value has a named Rust `AccountStatus` variant. The `Other(String)` fallback preserves **unrecognized future status strings**, with exact `Display` and JSON serialization. Wrong JSON types fail decoding. Required `status` cannot be missing/null; optional `crypto_status` can.

The default summary still displays plain wire values (e.g. `Status: ACTIVE`, `Crypto status: PAPER_ONLY`); `--out` remains the unmodified provider response. This is intentionally equivalent to Kirke; internals remain Rust-idiomatic.

### Intentionally deferred

- All documented account properties mapped to typed fields, their nullability, and historical/deprecated schema variants.
- Business-state diagnostics and explanations; color/TTY behavior; JSON stdout and Unix pipe conventions. Those are tracked as **cross-cutting M3 work**: [Medea #44](https://github.com/black-belt-badger/medea/issues/44) and [Kirke #44](https://github.com/black-belt-badger/kirke/issues/44).
- New request flags, trading writes, and assumptions that `ACTIVE` means unrestricted crypto trading.

This is a small parity change rather than closure of the entire account-fidelity issue.
