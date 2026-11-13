# Shared Kirke / Medea CLI and diagnostics contract

**Status: proposed v0.1 — reviewable design, not a claim that all flags exist.**
Tracks [Kirke #44](https://github.com/black-belt-badger/kirke/issues/44)
and [Medea #44](https://github.com/black-belt-badger/medea/issues/44).
This document is intentionally identical in both repositories. Each implementation
can be idiomatic; wire-visible arguments and outcomes should agree.

## Compatibility and staged rollout

Existing released commands retain their defaults, option names, stdout notices
and export behavior **until each endpoint is deliberately migrated**. In
particular, current `account --out` emits a path-only notice on stdout, while
crypto data `--format json` represents complete paginated page envelopes.
Do not silently reinterpret these as the final convention.

This PR introduces only **pure diagnostics models and account findings**:
no `--check`, `--format` or `--color` flags are added to account,
no exit statuses change, and no network or trading behavior is touched.

## Proposed data/format and Unix contract (for review)

| Aspect | Proposed observable behavior | Compatibility/migration |
| --- | --- | --- |
| Text | `--format text` on read commands; human summaries on stdout | Retain existing no-flag defaults until explicit staged migration |
| JSON | `--format json` means **raw provider JSON**, not normalized fields; valid single JSON document on stdout | Single-object and array endpoints first, starting with account; preserve precision and unknown fields |
| JSONL | `--format jsonl` only where each record has defined semantics; not for a single account | Existing crypto records stay symbolized; document that they omit provider envelope metadata and continuation tokens |
| Multi-page JSON | Existing bounded crypto page-envelope format is preserved, including pagination tokens | Do not silently turn it into one flat array or invent page tokens for trading orders |
| stdout | Requested data only; no ANSI, logs, progress or diagnostic records in machine streams | Historical path-only success notice in `--out` needs a deliberate migration |
| stderr | Sanitized errors, optional explicitly requested diagnostics/progress; never credentials, account IDs or response bodies | Diagnostic output opt-in; no default noise |
| `--out PATH` | Atomically replace a private file (0600 on Unix); failure leaves preexisting output untouched | Proposed future file mode emits **no stdout**; verbose status may go to stderr; existing notices retained until migration |
| Piping | `--format json \| jq` should parse; intentional consumer `EPIPE` (`\| head`) should not print a stack trace | Add integration fixtures in the endpoint migration PR |
| Color | `--color auto\|always\|never`; auto requires interactive TTY, and `NO_COLOR` disables auto | Never add ANSI to JSON/JSONL; even explicit always cannot color machine output |
| Help | Document only implemented flags; no promised flag should silently be accepted and ignored | Add flags incrementally and verify `--help` |
| Safety | Read-only diagnostics never submit/modify trades. Existing paper write commands remain dry-run-first and require explicit `--confirm` | No piped JSON or findings can implicitly authorize an order |

**Raw vs normalized:** `json` is the raw provider representation. If normalized
JSON is ever required, give it a separately versioned name/schema; never
silently normalize decimals or drop unknown fields under `--format json`.

## Proposed diagnostic contract (pure core implemented in this PR)

The diagnostic data model is independent from HTTP, domain decoding, terminals,
CLI parsing and file I/O:

| Field | Type | Meaning |
| --- | --- | --- |
| `code` | stable uppercase identifier | Machine identity of a factual finding; no dynamic account identifiers |
| `severity` | `info\|warning\|error` | Actionability, **not** request/HTTP success |
| `path` | dot-separated field path | E.g. `trading_blocked`, never an account number |
| `message` | short factual string | No secrets, server response bodies or trading advice |
| `explanation` | optional factual string | Clarifies known information and limits of inference |

Proposed request: `--check` displays findings on **stderr** while stdout
continues to carry only requested data. `--explain` adds factual explanation.
`--diagnostics text|json` selects only the **stderr diagnostic channel**;
JSON diagnostics should use one versioned JSON envelope per invocation, e.g.:

```json
{"schema_version":1,"findings":[{"code":"ACCOUNT_TRADING_BLOCKED","severity":"error","path":"trading_blocked","message":"Alpaca reports trading_blocked=true.","explanation":"This restriction is independent of status; an ACTIVE status does not override it."}]}
```

No raw account balances or private identifiers appear in findings. Proposed
`--fail-on error|warning` only makes the optional diagnostic threshold fail;
without it, a successful read with findings exits 0. An ERROR finding is a
business restriction, **not** a malformed API response.

### Account rule vocabulary implemented in both languages

| Code | Severity | Path | Condition |
| --- | --- | --- | --- |
| `ACCOUNT_STATUS_NOT_ACTIVE` | warning | `status` | Provider status differs from ACTIVE, without inferring permission |
| `ACCOUNT_TRADING_BLOCKED` | error | `trading_blocked` | Provider explicitly reports true, even with ACTIVE account |
| `ACCOUNT_TRADING_BLOCKED_UNKNOWN` | info | `trading_blocked` | Optional field is missing/null; do not assume false |
| `ACCOUNT_CRYPTO_STATUS_NOT_ACTIVE` | info | `crypto_status` | Provider reports different status; a PAPER_ONLY crypto status is **not automatically an error** |

`trading_blocked=false` alone is **not** proof that an order will be accepted.
Missing `crypto_status` does not create a supposed restriction. Account
status and crypto status remain separate typed values.

## Proposed exit code table (not applied yet)

| Status | Code |
| --- | ---: |
| Successful read/validated output, including ordinary findings | 0 |
| CLI usage, argument or cross-filter validation error | 2 |
| Missing/invalid credentials or rejected authentication | 3 |
| Network/timeout/transport failure | 4 |
| Provider HTTP failure (including 404 and 429) | 5 |
| Unexpected or malformed response/schema | 6 |
| Local file/pipe error not caused by consumer closing an intentional stdout pipe | 7 |
| Optional `--fail-on` diagnostic threshold reached | 8 |

Only validated **non-streaming** stdout broken pipes should be treated as
intentional consumer closure; an unrelated I/O error still fails. Account
404 and order-by-ID 404 remain distinguished by their sanitized messages.

## Review decisions required before flag implementation

1. Confirm the proposed `--check`, `--diagnostics`, `--explain`,
   `--fail-on` interaction and stderr-only diagnostic transport.
2. Confirm `--format json` as untouched provider JSON and a separate future
   name for normalized JSON.
3. Confirm how to migrate existing `--out` path notices away from stdout,
   without a silent breaking change.
4. Confirm exit codes, `NO_COLOR` behavior and broken-pipe handling.
5. Confirm dry-run-first semantics for all future trading writes.

## Incremental, reviewable implementation

1. **This paired PR:** identical contract, pure Finding/Severity types, stable
   account rules and synthetic parity tests; no observable CLI changes.
2. **Account slice:** opt-in JSON stdout (raw), checked stderr findings, help,
   exit/pipe/color and security integration fixtures after contract review.
3. **Read-only endpoints:** positions, assets, orders, order-get — use the
   shared presentation, diagnostic and output model.
4. **Crypto data streams:** preserve page envelopes and JSONL records,
   introduce safe broken-pipe behavior and bounded page rollback tests.
5. **Paper write commands:** maintain explicit confirmation and no unsafe
   automatic submission; audit all diagnostics and stdout paths.

The paired fixtures cover ACTIVE + trading_blocked=true, absent
trading_blocked, a non-ACTIVE account state, independent PAPER_ONLY crypto
state, explicit unblocked status, and exclusion of private fields.
No real Alpaca credentials or network calls are used.
