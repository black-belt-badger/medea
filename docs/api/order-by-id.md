# M3 — GET /v2/orders/{order_id} response and failure fidelity (Medea)

**Tracking:** [Medea M3 #37](https://github.com/black-belt-badger/medea/issues/37), paired with [merged Kirke PR #53](https://github.com/black-belt-badger/kirke/pull/53). The complete typed order and nested-leg schema was implemented in [Medea PR #52](https://github.com/black-belt-badger/medea/pull/52).

**Provider source:** [Alpaca Get Order by ID](https://docs.alpaca.markets/us/reference/getorderbyorderid-1). This is **not** the distinct by-client-order-ID operation.

## Request contract

| API property | Medea CLI or behavior | Validation |
| --- | --- | --- |
| Method / endpoint | GET https://paper-api.alpaca.markets/v2/orders/{order_id} | Fixed paper-only HTTPS host, no request body, shared authenticated HTTP client, 20-second timeout and no redirect following |
| Required path UUID | medea order get UUID | Opaque OrderId validated before credentials: exactly 36 ASCII bytes, four hyphens in canonical positions, hex elsewhere; URL path segment encoded |
| Optional nested | --nested | Omit by default; pass nested=true only when the switch is supplied; no unrelated query flags |
| Result shape | One JSON object | Shared fully typed decode_order; arrays/scalars, missing/blank ID or missing/invalid status fail |
| Identity check | Response primary id equals requested order ID | Case-insensitive ASCII UUID comparison, preserving original response; mismatched IDs fail even on HTTP 200 |
| Human output | medea order get UUID | Reuse the orders summary fields and typed child leg ratios, exact financial strings, null handling |
| Raw export | medea order get UUID --out PATH | Only after the decoded ID matches; save the **original** provider response bytes atomically with owner-only permissions; retain unknown properties and full decimal spelling |

An unknown UUID cannot reach the network through another CLI flag or user-supplied path fragment. A response for an unrelated order is never displayed or exported.

## Response coverage

The shared [Orders response matrix](orders-response.md) covers all **37/37 published Order** and **35/35 OrderLeg** fields. This includes partial equity fills, crypto notional orders with null quantity, bracket and option multi-leg orders, trailing stops, replacement IDs, all published timestamps and open typed enums.

- Child legs are requested through --nested. A parent multi-leg option order may have no side; leg sides, exact ratios and status appear in the readable summary. Grandchild order arrays are not silently discarded: nonempty grandchildren are invalid per the current Kirke/Medea typed contract.
- Numbers represented as provider decimal strings remain strings, never floating-point values. The original byte buffer is preserved even when the typed summary omits future extension properties, null-vs-absent information or long numeric literals.
- The list operation and by-ID operation share a typed Order decoder, but differ in the required top-level response shape and ID-matching requirement.

## Error contract

| Outcome | Diagnostic / behavior |
| --- | --- |
| 200 valid matching Order | Summary, or atomic private JSON export |
| 404 | Alpaca paper order not found (HTTP 404). / nonzero process exit |
| 401 | Alpaca paper API rejected the credentials (HTTP 401). |
| 403 / 429 / 5xx | Alpaca paper API returned HTTP CODE. |
| Transport / timeout | Could not connect to Alpaca's paper API. Check your network connection. |
| 200 malformed / array / bad scalar / invalid nested Order | Invalid Alpaca order response (expected one valid order object). |
| 200 valid order with unexpected id | Invalid Alpaca order response: order ID does not match requested UUID. |

All failure messages are intentionally **sanitized**: no provider body, returned account/order ID, request headers, secrets or parsing exceptions. No bad or mismatched response is saved to the output path. The HTTP 404-to-not-found distinction is already implemented in the shared client and is retained defensively by the pure lookup boundary.

## Tests and live verification

The offline tests use unchanged **synthetic** Kirke fixtures for partial-fill equity, crypto notional orders and nested option legs, plus adversarial malformed JSON/IDs. They verify unchanged raw bytes, case-insensitive identity, mismatches, non-array shapes and wrong JSON types, 404 versus 401/403/429/503, transport diagnostics, validated URL path and nested defaults. No credentials or real trading account information are used in CI.

The opt-in read-only script ./scripts/cd.sh takes an order ID from the authenticated paper list, performs both ordinary and nested by-ID lookups, checks the JSON primary id matches the requested server id and verifies private file permissions.

**Explicit triparity difference:** Atalanta currently models an order-by-ID 404 as a successful optional absence (Result.Ok(Optional.empty())), whereas Kirke and Medea are CLIs and use a nonzero not-found diagnostic. This is recorded for the [Atalanta triparity audit #26](https://github.com/black-belt-badger/atalanta/issues/26), **not** reconciled by changing Atalanta or the CLI return conventions in this PR.

**Deferred:** changing human output defaults, adding explicit nested=false, global stdout/stderr/diagnostics/pipelines, colors, shell completion or order-write commands. Shared CLI design remains in [Medea #44](https://github.com/black-belt-badger/medea/issues/44) / [Kirke #44](https://github.com/black-belt-badger/kirke/issues/44). Keep #37 open pending stakeholder acceptance.

## Endpoint-level acceptance and shared Kirke/Medea behavior

The API codec and HTTP tests above cover the provider contract. The final
M3 check exercises the complete **lookup, identity validation, rendering and
private export** boundary with the same fabricated Order fixtures in Haskell
and Rust, without any credentials or live HTTP.

| Invocation or result | Contract shared by both CLIs |
| --- | --- |
| `order get --help` | Lists UUID, `--nested` and `--out PATH`; no network |
| `order get UUID` | Read-only authenticated paper GET for exactly that validated UUID; no arbitrary query |
| `order get UUID --nested` | Sends only `nested=true`; default omits `nested`; full child order list decoded |
| Invalid/missing UUID, unsupported flags | CLI error before credentials or network |
| Valid matching order | Exact decimal summary using the same Order model and renderer as list-orders |
| Crypto notional, partially filled equity, nested multi-leg options | Nullable fields and long decimal quantities preserved; option leg ratios displayed |
| `order get UUID --out existing-dir/order.json` | Validate complete order and matching ID first; atomically export original provider bytes with private owner-only mode (0600 on Unix), then path-only notice |
| HTTP 404 | Distinct nonzero not-found error, not a successful empty JSON object |
| HTTP 401/403/429/5xx or transport failure | Nonzero sanitized error without request secrets or provider body |
| Valid HTTP 200 with different primary order ID | Fixed sanitized mismatch diagnostic; no stdout and no file replacement |
| Malformed JSON, unexpected array/scalar, missing fields, incorrect decimal types or malformed legs | Fixed invalid-order diagnostic; no output file mutation |
| Missing/unwritable export directory | Nonzero error; no success notice or order payload on stdout |

The command-level offline tests in both repositories reuse the **identical**
`order-equity.json`, `order-crypto.json` and `order-mleg.json` synthetic
fixtures. They verify matching response identity, exact full-precision output,
leg rendering, raw byte fidelity, replacement permissions, failure-message
sanitization and the critical guarantee that 404, mismatched identities or
invalid JSON **cannot overwrite an existing export**.

The endpoint intentionally does not implement `orders:by_client_order_id`,
modify trading state or introduce additional CLI conventions. Shared
diagnostics, color, shell completion, output modes and Unix pipeline behavior
remain reserved for Kirke/Medea #44; Atalanta's different library-level
404-as-optional result remains documented rather than silently changed.
