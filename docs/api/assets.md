# M3 — GET /v2/assets API coverage (Medea/Rust)

Scope: [Medea #34](https://github.com/black-belt-badger/medea/issues/34), paired with merged [Kirke PR #47](https://github.com/black-belt-badger/kirke/pull/47).

Source verified 2026-11-13: [Alpaca GET /v2/assets](https://docs.alpaca.markets/us/reference/get-v2-assets-1) and the [machine-readable OpenAPI](https://docs.alpaca.markets/us/reference/get-v2-assets-1.md).

**Coverage:** all four documented request parameters, all 19 documented response properties, all response enum values and forward-compatible unknown strings. This PR is the API-fidelity pass; shared terminal colors, diagnostics and Unix output behavior remain deferred.

## Request contract

| Parameter | Alpaca behavior | Medea CLI |
| --- | --- | --- |
| `status` | Optional; omitted returns all statuses | `--status active\|inactive\|all`; omitted sends no parameter |
| `asset_class` | Optional, provider default `us_equity` | `--asset-class CLASS`; Medea intentionally **defaults to crypto**, explicitly sending `asset_class=crypto` |
| `exchange` | Comma-separated exchange filter for equities and indexes | `--exchange NASDAQ,NYSE` or `--asset-class us_index --exchange INDEX` |
| `attributes` | Comma-separated list of attributes, matched with **OR** semantics (any match) | `--attributes has_options,ipo` |

The request is an authenticated read-only `GET https://paper-api.alpaca.markets/v2/assets` with no body. Requests are built using encoded URL query pairs; user input never becomes raw path segments. Empty, duplicate and unknown CSV tokens are rejected. Spaces around commas are accepted.

**Ten asset-class values:** `us_equity`, `us_option`, `crypto`, `crypto_perp`, `treasury`, `corporate`, `global_equity`, `us_index`, `us_equity_chain`, `ipo`.

**Nine exchange filter values:** `AMEX`, `ARCA`, `BATS`, `NYSE`, `NASDAQ`, `NYSEARCA`, `OTC`, `TXSE`, `INDEX`. `INDEX` requires `us_index`; for `us_equity`, it is rejected. With `us_index`, only `INDEX` is accepted. All other asset classes reject exchange filters, rather than silently ignoring an ineffective parameter. Exchange values appearing in **responses** include crypto venues and are a deliberately broader enum.

**Eight attribute filter values:** `ptp_no_exception`, `ptp_with_exception`, `ipo`, `has_options`, `options_late_close`, `fractional_eh_enabled`, `overnight_tradable`, `overnight_halted`.

Class/exchange cross-field validation happens **before credentials are loaded or HTTP is attempted**. `--asset-class ipo` and `--attributes ipo` are distinct operations.

```sh
medea assets --asset-class us_equity --status active \
  --exchange NASDAQ,NYSE --attributes has_options,overnight_tradable \
  --out output/assets.json
```

## Response contract

Alpaca returns an array; `[]` is a valid response. The OpenAPI schema requires ten properties. Conditional response properties are optional and may be null.

| Alpaca property | Rust model type | Requirement |
| --- | --- | --- |
| `id` | `String` | Required |
| `class` | `AssetKind (open enum)` | Required |
| `exchange` | `AssetVenue (open enum)` | Required |
| `symbol` | `String` | Required |
| `name` | `String` | Required |
| `status` | `AssetState (open enum)` | Required |
| `tradable` | `bool` | Required |
| `marginable` | `bool` | Required |
| `shortable` | `bool` | Required |
| `fractionable` | `bool` | Required |
| `attributes` | `Option<Vec<AssetAttributeValue>>` | Optional |
| `borrow_status` | `Option<BorrowStatus>` | Optional |
| `cusip` | `Option<String> (nullable)` | Optional |
| `maintenance_margin_requirement` | `Option<serde_json::Number> (deprecated JSON number)` | Optional |
| `margin_requirement_long` | `Option<String> (exact decimal)` | Optional |
| `margin_requirement_short` | `Option<String> (exact decimal)` | Optional |
| `min_order_size` | `Option<String> (exact decimal)` | Optional |
| `min_trade_increment` | `Option<String> (exact decimal)` | Optional |
| `price_increment` | `Option<String> (exact decimal)` | Optional |

### Enum and numeric safety

- Class: all ten known values typed under `AssetKind` / `KnownAssetKind`; unknown future class values become `Other(String)`.
- Exchange: all 13 known provider values (including crypto venues `CRYPTO`, `ERSX`, `CRXL`, `ASCX`) plus unknown future values.
- Status: `active`, `inactive`, or an unknown future string.
- Attributes: eight known values or an unknown future string.
- Borrow status: `easy_to_borrow`, `hard_to_borrow`, or an unknown future string.
- Numeric **strings** (`min_order_size`, `min_trade_increment`, `price_increment`, `margin_requirement_long`, `margin_requirement_short`) are preserved as `String` with exact formatting and precision. The single deprecated numeric field `maintenance_margin_requirement` uses `serde_json::Number` with `arbitrary_precision`; no f64 conversion.
- Unexpected enum strings are allowed but wrong JSON types are rejected. Required properties must be present and non-null.
- Unknown upstream properties do not break typed decoding and are preserved **byte-for-byte** when `--out` saves the unmodified API response (atomic, owner-only mode). A recent crypto response included undocumented `easy_to_borrow` (boolean); it remains an unknown nonrequired field, not confused with `borrow_status`.

## CLI parity and checks

`medea assets` still defaults to **crypto**, as Kirke does, rather than Alpaca's own **us_equity** default. Existing status, output and summary conventions remain unchanged. The human summary shows symbol, class, status, name, tradability and fractionability. `--out PATH` continues to save full original provider JSON to private files.

Offline tests cover all ten asset classes, nine valid query exchanges, eight attributes, CSV boundaries, HTTP encoding, cross-field validation and missing credentials, all 19 response properties, strict required types, optional/null properties, all enum families with forward compatibility, full-precision numerics, malformed responses and empty lists.

Opt-in `./scripts/cd.sh` tests the real paper API, including a NASDAQ/has_options filtered request, checks the result shape and private file permissions. No personal account data belongs in CI or fixtures.

**Deferred:** UUID/date format validation, alternate output formats, shell completion, business interpretation, color, diagnostics, Unix pipeline conventions. See [Medea #44](https://github.com/black-belt-badger/medea/issues/44) and [Kirke #44](https://github.com/black-belt-badger/kirke/issues/44). Keep the parent endpoint M3 issue open for final review.

## Offline end-to-end acceptance and CLI examples

The application deliberately retains the established **crypto-first** default (matching
Kirke). Alpaca itself defaults to `us_equity` when `asset_class` is absent, so
Medea always sends `asset_class=crypto` for `medea assets`. The default is a
consumer policy, not an implicit Alpaca API default.

| Example / condition | Contract |
| --- | --- |
| `medea assets --help` | Exit 0, help with all four request filters and `--out`; no credentials or HTTP |
| `medea assets` | Authenticated paper-only GET with `asset_class=crypto`; concise human summary |
| `medea assets --asset-class us_equity --status inactive --exchange NASDAQ,NYSE --attributes has_options,ipo` | All four parameters encoded once with comma-separated lists; no dropped filters; any-match attribute semantics are applied by Alpaca |
| `medea assets --asset-class us_index --exchange INDEX` | Allowed; other exchange filters for `us_index`, or any exchange filter for crypto, rejected before credentials |
| `medea assets --out existing-dir/assets.json` | Validate entire array before output; save original bytes atomically (0600 on Unix); print success notice only |
| API returns `[]` | Exit 0; concise `No matching assets.` summary, or original `[]` export |
| Missing credentials, unsupported flags, malformed/duplicate CSV, repeated filter flags | Nonzero exit; diagnostics on stderr without provider data |
| Invalid JSON, non-array response, null element, wrong field type, missing required property | Nonzero exit; fixed invalid-response message; no stdout and prior export unchanged |
| HTTP 401/403/429/5xx, request timeout, or connection failure | Nonzero exit; sanitized error with no credentials or upstream response body |
| Export directory missing/unwritable | Nonzero exit; no success notice or partial destination file |

The offline tests in `src/assets_acceptance_tests.rs` exercise the actual
summary/export boundary using synthetic crypto and unknown future asset records,
arbitrarily precise numbers, private source IDs, extra fields, `[]`, malformed
responses, preexisting destination files and Unix owner-only permissions.
A separate client test passes real parsed CLI options through the HTTP request
builder, explicitly checking the crypto-first default and preservation of every
supplied filter; CLI integration tests reject unsupported and repeated flags
before loading credentials.

These tests use no real Alpaca secrets and never call the network.
Shared Kirke/Medea diagnostics, shell completion, redaction modes, terminal
colors and Unix pipeline policy remain in [#44](https://github.com/black-belt-badger/medea/issues/44).
