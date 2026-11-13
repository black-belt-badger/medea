# Medea — crypto paper-order submission (M5)

**Endpoint:** `POST https://paper-api.alpaca.markets/v2/orders`. This is a **paper-only write operation** and is never sent automatically. It does not support live trading or a configurable host.

Tracks [Medea #14](https://github.com/black-belt-badger/medea/issues/14) and follows the tested [Kirke #20](https://github.com/black-belt-badger/kirke/issues/20) contract. See [Alpaca Create Order](https://docs.alpaca.markets/us/reference/postorder) and [Crypto Orders](https://docs.alpaca.markets/us/docs/crypto-orders).

## Usage

```sh
# Safe offline dry run: no credentials loaded, no API call
medea order submit --symbol BTC/USD --side buy --type market \
    --qty 0.0001 --client-order-id experiment-btc-001

# Preview a limit request
medea order submit --symbol BTC/USD --side buy --type limit \
    --qty 0.0001 --limit-price 65000.00 --time-in-force gtc \
    --client-order-id experiment-limit-001

# Explicit paper-account write; requires paper credentials
medea order submit --symbol BTC/USD --side buy --type market \
    --qty 0.0001 --client-order-id experiment-btc-001 --confirm

# Confirmed notional buy with private, atomic response export
medea order submit --symbol ETH/USD --side buy --type market \
    --notional 25.12 --client-order-id experiment-eth-001 \
    --confirm --out output/paper-order.json
```

These are syntax examples, **not evidence that an order was submitted**. The `--out` directory must already exist.

The default is dry run: the validated JSON request is printed without checking credentials, opening a network connection, or writing files. **Only `--confirm`** permits one paper POST. `client_order_id` is a caller-selected stable identifier; Alpaca may reject duplicates, but it is **not a guaranteed exactly-once execution mechanism**. Never blindly retry an ambiguous submission.

## Typed request and validation

| Argument | Behavior |
| --- | --- |
| `--symbol PAIR` | Required one uppercase alphanumeric crypto pair with `/` (e.g. `BTC/USD`, `ETH/BTC`); no CSV, path injection or fragments |
| `--side buy\|sell` | Required typed enum, exactly two supported values |
| `--type market\|limit` | Required typed enum, no stop, brackets or options |
| `--time-in-force gtc\|ioc` | Default `gtc`, no unsupported `day` or `fok` |
| `--qty DECIMAL` | Positive string decimal, up to 9 fractional places, no floating-point conversion |
| `--notional DECIMAL` | Market only; USD-quoted crypto limited to 2 fractional places, others up to 9 |
| `--limit-price DECIMAL` | Required for limit orders, forbidden for market |
| `--client-order-id ID` | Required, 1–128 ASCII letters/digits plus `.`, `-`, `_`, `:`; never auto-generated |
| `--confirm` | Absent by default; explicit switch enabling paper POST |
| `--out PATH` | Private response export **only with** `--confirm` |

Exactly one of `--qty` or `--notional` is required. A limit order needs quantity **and** price; a market order accepts quantity or notional and has no limit price. Zero/negative values, exponent notation, non-ASCII digits, excessive fractional precision, empty IDs, or unsafe symbols fail **before credential loading**. Provider-side checks such as buying power, asset availability, minimum size and increment still apply.

Rust `SubmitSymbol`, `SubmitAmount` and `ClientOrderId` validate textual inputs via `FromStr`. `SubmitSide`, `SubmitKind` and `SubmitTimeInForce` use typed enums. A `SubmitOrderArgs::prepare` result contains only the validated payload; credentials, confirmation and output destination are not serialized.

For example, a dry-run market buy serializes to:

```json
{
  "symbol": "BTC/USD",
  "side": "buy",
  "type": "market",
  "time_in_force": "gtc",
  "client_order_id": "experiment-btc-001",
  "qty": "0.0001"
}
```

Financial amounts are **JSON strings**, not `f64`. The exact user-supplied decimal spelling is preserved. Missing alternatives are omitted, not sent as null. Alpaca's default simple `order_class` is left unspecified.

## Paper transport and result handling

- HTTPS target is fixed at `paper-api.alpaca.markets`, with the exact path `/v2/orders`; no configurable live-trading URL.
- Credential headers use the existing authenticated client. Redirects are disabled; the client has a 20-second timeout and **never automatically retries a POST**. The low-level Rust POST builder also checks the explicit confirmation bit, so valid dry-run requests cannot bypass the CLI's first safety gate.
- A successful HTTP 200/201 response is decoded with Medea's existing typed `Order` model and verified against the caller's **exact `client_order_id`**. Unknown response fields remain in the unmodified original bytes, saved only when `--out` is requested.
- On successful HTTP receipt, the CLI explicitly states that **a fill is NOT confirmed**. Inspect order status with `medea order get ORDER_ID` or `medea orders`.
- Missing/invalid provider response or mismatched `client_order_id` becomes a sanitized error. It never echoes full provider JSON, credentials, or an unexpected order identifier.
- HTTP 401, 403, 422 and 429 have distinct actionable errors. For 429, 5xx, or a network failure, **the order may still have been accepted**; inspect existing paper orders by client ID before considering any retry. For response reconstruction after an uncertain outcome, Alpaca provides [GET /v2/orders:by_client_order_id](https://docs.alpaca.markets/us/reference/getorderbyclientorderid). This CLI does not yet expose that additional endpoint.
- A successful validated raw response exported with `--out` uses the existing atomic owner-only file writer. A failed/invalid submission response is never written.

## Testing and limits

Offline tests cover both request and result types; valid and invalid numeric precision and CLI forms; exact serialization; dry-run with **all credentials removed**; `--confirm` reaching credential validation but **never a test network POST**; paper-only method/host/headers/body; success with matching client ID; malformed and mismatched responses; and HTTP failure classes. CI and release build require no real broker credentials.

This intentionally small M5 endpoint supports **crypto market and limit paper orders**. It does not implement live orders, equities/options/brackets, stop-limit, cancel/replace, auto-reconciliation, retries, interactive confirmations, or global CLI-color/piping conventions. Medea's cancellation endpoint is a separate issue (#15). The architectural and triparity reviews remain deferred until endpoint work is completed.
