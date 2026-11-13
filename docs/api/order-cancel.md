# Cancel one Alpaca paper order — Medea

**Endpoint:** `DELETE https://paper-api.alpaca.markets/v2/orders/{order_id}`, matching the merged [Kirke PR #56](https://github.com/black-belt-badger/kirke/pull/56). This endpoint **changes paper-account state** and is never directed to the live trading host.

## Default is a safe offline preview

```sh
# Without credentials or network access; does not cancel anything
medea order cancel 9275f66a-0a1b-4a22-8c55-167416c7cf14

# Only this explicit flag sends one real paper-account DELETE request
medea order cancel ACTUAL_SERVER_ORDER_UUID --confirm

# Check final state separately (read-only)
medea order get ACTUAL_SERVER_ORDER_UUID
```

The first UUID is a synthetic example. Use the **server-issued `id`**, not `client_order_id`. A valid order ID has 36 ASCII characters in canonical UUID form. Invalid UUIDs, path/query injection, `--all`, `--out` and other unrelated flags fail during CLI parsing, before credentials or networking.

There are **two confirmation gates**: the dry-run application boundary does not load credentials or call the HTTP callback, and the low-level Rust request builder returns `ConfirmationRequired` if called directly without `confirmed: true`. Confirmed execution sends **exactly one request**; there is no automatic application retry, cancel-all command, credential fallback to live trading, or background polling. The 20-second HTTPS transport forbids redirects.

## Request and response contract

| Component | Behavior |
| --- | --- |
| Method | `DELETE`, exactly one order |
| Host | `paper-api.alpaca.markets` exclusively, HTTPS port 443 |
| Path | `/v2/orders/{order_id}` using a validated, encoded UUID path segment |
| Query/body | Neither |
| Authentication | Shared paper API headers, loaded **only** on `--confirm` |
| 204 | Cancellation **request accepted**, not a confirmed canceled state |
| 404 | `Alpaca paper order not found (HTTP 404).` |
| 422 | `Alpaca rejected cancellation: order is not cancelable (HTTP 422).` |
| 401 | Credentials rejected; distinct diagnostic |
| 403 / 429 / 5xx | Permission, rate-limit or provider failures, distinct HTTP status |
| Unexpected 200 / 201 / 202 | Not treated as cancellation accepted |
| Transport failure or timeout | Uncertain outcome; never blindly retry |

An order can fill while a cancellation is pending. After HTTP 204, **query the order with `medea order get` and inspect its actual status**. If a timeout occurs, inspect status before considering any repeat DELETE. No response or error body is parsed or printed during cancellation; credential headers, raw HTTP exceptions, provider body and sensitive values are never logged by this command.

## Rust contract and testing

- `OrderCancelArgs` contains the same validated `OrderId` type used by `order get` and an explicit `confirmed: bool` flag.
- `run_cancel_with` separates the dry-run decision from the side effect, accepting a `FnOnce` callback. An unconfirmed invocation never calls it; a confirmed invocation calls it once only.
- `cancel_order_request` separately rejects unconfirmed DELETE construction and produces the exact authenticated, paper-only, bodyless request.
- `interpret_cancel_status` accepts exactly HTTP 204; 404/422 are dedicated typed errors and other statuses remain sanitized.
- Offline unit and executable CLI tests cover preview without credentials, rejection of unsafe arguments, confirmed credential requirements, request safety, all status branches, callback counts, and redaction.
- `./scripts/cd.sh` executes **only** the synthetic no-credentials preview. It never calls `order cancel --confirm`; no automated tests issue real POST or DELETE operations.

This matches [Kirke #21](https://github.com/black-belt-badger/kirke/issues/21). Any subsequent Atalanta–Kirke–Medea API/CLI refinements should be explicitly agreed in the planned triparity review, not silently folded into this endpoint.

**Reference:** [Alpaca Delete Order by ID](https://docs.alpaca.markets/us/reference/deleteorderbyorderid-1).
