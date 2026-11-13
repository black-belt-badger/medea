# M3 — GET /v2/orders request-parameter fidelity (Medea / Rust)

Scope: [Medea #36](https://github.com/black-belt-badger/medea/issues/36); paired merged implementation [Kirke PR #51](https://github.com/black-belt-badger/kirke/pull/51). This is **request phase 1**, not final endpoint acceptance.

Reference verified 2026-11-13: [Alpaca GET /v2/orders](https://docs.alpaca.markets/us/reference/getallorders-1) and [machine-readable OpenAPI](https://docs.alpaca.markets/us/reference/getallorders-1.md).

## Request contract: all 11 documented parameters

The endpoint is an authenticated, **read-only** `GET https://paper-api.alpaca.markets/v2/orders`. There are no path arguments or request bodies; a successful response is a JSON array. Medea constructs URL query pairs through reqwest, which percent-encodes reserved characters.

| Provider query | Semantics | Medea CLI |
| --- | --- | --- |
| `status` | `open`, `closed`, `all`; provider default `open` | `--status open|closed|all`; Medea explicitly sends `open` by default |
| `limit` | 1–500, provider default 50 | `--limit N`; omitted by default |
| `after` | Exclusive lower submission-time bound | `--after RFC3339`, timezone required |
| `until` | Exclusive upper submission-time bound | `--until RFC3339`, timezone required |
| `direction` | `asc`, `desc`; provider default `desc` | `--direction asc|desc`; Medea explicitly sends `desc` by default |
| `nested` | Roll multi-leg orders up under a parent with a `legs` list | `--nested`; sends `true` if present, otherwise omitted |
| `symbols` | CSV trading symbols, including `BTCUSD` (no crypto slash) | `--symbols BTCUSD,AAPL`; validated |
| `side` | Filter buy/sell orders | `--side buy|sell`; typed enum |
| `asset_class` | CSV asset classes, plus special `all` | `--asset-class crypto,us_equity`; typed, validated CSV |
| `before_order_id` | Exclusive server-order-UUID pagination | `--before-order-id UUID` |
| `after_order_id` | Exclusive server-order-UUID pagination | `--after-order-id UUID` |

**Asset-class vocabulary (10):** `us_equity`, `us_option`, `crypto`, `crypto_perp`, `treasury`, `corporate`, `global_equity`, `us_index`, `us_equity_chain`, `ipo`. A special `all` is valid by itself, but cannot be combined with any individual class. Empty, duplicated and unknown CSV tokens are rejected; spaces around commas are accepted.

**Order submission times** must be timezone-qualified RFC3339 values; no date-only crypto-market-data shorthand. `OrderInstant` preserves the exact input spelling (including nanoseconds and the timezone offset) and compares absolute instants to reject reversed bounds. The API interprets both `after` and `until` exclusively.

**Pagination rules:** `--before-order-id` and `--after-order-id` are mutually exclusive and cannot be combined with either `--after` or `--until`. Both use the existing validated server-issued canonical UUID type (not `client_order_id`). There is **no page token** and no implicit multi-page iteration. Invalid combinations are rejected **before credentials are loaded or HTTP is attempted**.

## Examples

Read the latest open orders (existing safe defaults):

```sh
medea orders
```

Retrieve up to ten crypto buy orders, nesting any multi-leg children, and save the original provider JSON privately:

```sh
medea orders --status all --limit 10 --side buy \
  --asset-class crypto --nested --out output/orders.json
```

Bound an order search to an exclusive submission-time window and two asset classes:

```sh
medea orders --status all --asset-class crypto,us_equity \
  --after 2026-11-01T00:00:00Z --until 2026-11-14T00:00:00Z
```

Navigate manually from a server-issued order ID:

```sh
medea orders --status all --limit 10 \
  --before-order-id 9275f66a-0a1b-4a22-8c55-167416c7cf14
```

The example UUID is synthetic, not an account-specific value. Use `--after-order-id` as the alternative cursor direction.

## Security, testing, and follow-up

- Only the authenticated HTTPS paper host is used; no live-trading host, order placement, cancellation, or modifications.
- Existing output behavior is unchanged. Human summaries keep exact string amounts and prices; `--out PATH` writes the **complete original JSON bytes** atomically, with owner-only permissions. Future fields remain present.
- Offline tests cover all 11 query parameters and exact encoding, both anchors, all ten classes, CSV boundaries, time-zone offsets and nanosecond timestamps, incompatible filters, defaults and CLI errors before credentials.
- Opt-in `./scripts/cd.sh` validates a live bounded paper request with `--side buy --asset-class crypto --nested`, JSON array shape and private file mode. Tests in public CI never need credentials.

**Deferred response phase:** the published `Order` schema has 37 fields and nested order legs; this PR intentionally does **not** expand the typed order response. Kirke and Medea will receive paired response-schema PRs after request parity. Keep the parent #36 issue open.

Shared CLI colors, diagnostics, Unix pipeline conventions and shell-completion design remain deferred to [Medea #44](https://github.com/black-belt-badger/medea/issues/44) and [Kirke #44](https://github.com/black-belt-badger/kirke/issues/44).
