# M3 — GET /v2/positions API fidelity (Medea / Rust)

Scope: [Medea #35](https://github.com/black-belt-badger/medea/issues/35); paired implementation [Kirke PR #50](https://github.com/black-belt-badger/kirke/pull/50), already merged.

Reference checked 2026-11-13: [Alpaca GET /v2/positions](https://docs.alpaca.markets/us/reference/getallopenpositions), including its [machine-readable OpenAPI definition](https://docs.alpaca.markets/us/reference/getallopenpositions.md).

## Request contract

| Detail | Behavior |
| --- | --- |
| Method and URL | Authenticated `GET https://paper-api.alpaca.markets/v2/positions` (paper-only) |
| Path/query/body arguments | **None**. No `--symbol`, asset-class, history, pagination or limit filters. |
| Scope | All currently open positions; **not** closed-position history |
| Response | JSON array; `[]` means no open positions |
| CLI | `medea positions` or `medea positions --out PATH` |
| Security | Private credentials, read-only operation, no order or position mutations |
| Errors | Sanitized HTTP/transport errors and invalid response, no raw private bodies in terminal errors |

The single-position `GET /v2/positions/{symbol_or_asset_id}` is a **separate endpoint**, not an argument to this list command.

## Position response: all 22 documented properties

There are **17 required properties** and five optional properties. `Position` decodes every published field with the documented JSON type. An absent/null required property is an error. Missing or explicit-null optional fields become `None`; the unmodified raw output still distinguishes those wire states.

| JSON property | Rust type | Schema |
| --- | --- | --- |
| `asset_class` | `AssetKind` | Required |
| `asset_id` | `String` | Required |
| `asset_marginable` | `bool` | Required |
| `avg_entry_price` | `String` | Required |
| `avg_entry_swap_rate` | `Option<String>` | Optional |
| `change_today` | `String` | Required |
| `cost_basis` | `String` | Required |
| `current_price` | `String` | Required |
| `exchange` | `AssetVenue` | Required |
| `lastday_price` | `String` | Required |
| `market_value` | `String` | Required |
| `prev_swap_rate` | `Option<String>` | Optional |
| `qty` | `String` | Required |
| `qty_available` | `Option<String>` | Optional |
| `side` | `PositionSide` | Required |
| `swap_rate` | `Option<String>` | Optional |
| `symbol` | `String` | Required |
| `unrealized_intraday_pl` | `String` | Required |
| `unrealized_intraday_plpc` | `String` | Required |
| `unrealized_pl` | `String` | Required |
| `unrealized_plpc` | `String` | Required |
| `usd` | `Option<UsdPositionValues>` | Optional (LCT accounts) |

### USD values for local-currency trading accounts

The optional `usd` object has ten properties. When present, **only `avg_entry_price` and `cost_basis` are required**; the other eight may be omitted or null. These are USD-denominated values in addition to the position's own values. No automatic currency conversion, inferred exchange rate or portfolio aggregation is performed.

| USD property | Rust type | Schema |
| --- | --- | --- |
| `avg_entry_price` | `String` | Required |
| `change_today` | `Option<String>` | Optional |
| `cost_basis` | `String` | Required |
| `current_price` | `Option<String>` | Optional |
| `lastday_price` | `Option<String>` | Optional |
| `market_value` | `Option<String>` | Optional |
| `unrealized_intraday_pl` | `Option<String>` | Optional |
| `unrealized_intraday_plpc` | `Option<String>` | Optional |
| `unrealized_pl` | `Option<String>` | Optional |
| `unrealized_plpc` | `Option<String>` | Optional |

## Typed enum and exact-money rules

- `asset_class` reuses the existing `AssetKind` provider enum: all ten known asset classes plus `Other(String)` fallback for new class names.
- `exchange` reuses `AssetVenue` with all current provider exchanges and `Other(String)` fallback. The positions schema allows an **empty string** exchange for some instruments; it is now explicitly `KnownAssetVenue::NoExchange`.
- `side` is a new `PositionSide` enum with typed `long` / `short` and future `Other(String)` values. All enum JSON values must be strings.
- Quantities, swap rates, P/L amounts, fractional percentage changes, prices and cost basis remain exact **JSON strings** (`String`), never `f32`/`f64` or rounded display amounts.
- Present `"0"` remains distinct from an omitted/explicit-null optional value; booleans must be actual JSON booleans.
- Future unknown provider properties do not break typed parsing and are preserved **byte-for-byte** by the existing private `--out` export. The `usd` field does not overwrite top-level native-currency fields.

## CLI compatibility, testing and limits

The human-readable summary remains unchanged for complete valid positions: symbol, asset class, long/short side, exact quantity, market value, current price and unrealized P/L. Empty portfolio output remains `No open positions.`. The complete provider JSON is saved privately and atomically with `medea positions --out PATH`, with owner-only file permissions.

Offline tests cover 22/22 top-level and 10/10 nested fields; all 17 required properties; missing vs null vs present-zero; nested USD only-two-required validation; long fractional crypto quantities; negative prices/values; synthetic equity/crypto/option examples; documented empty exchange; future enum strings; wrong JSON types; malformed/non-array responses; no-filter request contract and unsupported CLI flags. No real account data or secrets belong in CI.

The opt-in `./scripts/cd.sh` exercises the authenticated paper API and checks actual positions JSON wire types and file permissions.

**Deferred intentionally:** summary layout changes, portfolio totals/aggregation, FX-normalized figures, new CLI flags, shell completion, terminal colors, diagnostics, JSON stdout/pipeline redesign and deeper validation of UUID/financial semantics. Cross-cutting design remains [Medea #44](https://github.com/black-belt-badger/medea/issues/44) and [Kirke #44](https://github.com/black-belt-badger/kirke/issues/44). Keep parent M3 positions issue open until final product-owner review.
