# M3 — GET /v2/orders complete response decoding (Medea / Rust)

Scope: [Medea M3 #36](https://github.com/black-belt-badger/medea/issues/36). Paired with merged [Kirke response PR #52](https://github.com/black-belt-badger/kirke/pull/52) and the earlier [Medea request PR #51](https://github.com/black-belt-badger/medea/pull/51).

Reference reviewed 2026-11-13: [Alpaca Get All Orders](https://docs.alpaca.markets/us/reference/getallorders-1), [Get Order by ID](https://docs.alpaca.markets/us/reference/getorderbyorderid-1) and the published [machine-readable OpenAPI](https://docs.alpaca.markets/us/reference/getallorders-1.md). This is a **read-only paper account** feature.

## Observable API behavior

The same decoder is used for both listing and by-ID lookups:

| Operation | Wire response | Medea API | Output |
| --- | --- | --- | --- |
| \`GET /v2/orders\` | Array of orders, possibly \`[]\` | \`decode_orders(&[u8])\` | Concise multi-order summary, or original JSON bytes with \`--out\` |
| \`GET /v2/orders/{order_id}\` | Exactly one order object | \`decode_order(&[u8])\` | Concise single-order summary, or original JSON bytes with \`--out\` |

The commands reject malformed/nonmatching JSON roots. The by-ID endpoint's HTTP 404 handling remains distinct from JSON decoding errors. The API does not create, change, or cancel orders.

## Complete published response coverage

The \`Order\` model contains 17 common summary/identity fields, 22 additional typed or precisely represented fields in flattened \`OrderDetails\`, and the \`legs\` field. Combined with the overlapping shapes, all **37/37 published \`Order\` properties** and all **35/35 published \`OrderLeg\` properties** are represented; independently verified against the current OpenAPI. Medea also accepts the live-provider extension fields \`source\` and \`subtag\` without making them mandatory.

| Group | Exact published fields | Rust representation |
| --- | --- | --- |
| Identity and classification | \`id\`, \`client_order_id\`, \`asset_id\`, \`asset_class\`, \`symbol\`, \`side\`, \`type\`, \`status\` | Required nonblank \`id: String\` and typed \`OrderState\`; others nullable/optional, with typed \`OrderSide\`, \`OrderType\` |
| Quantity and execution | \`qty\`, \`notional\`, \`filled_qty\`, \`filled_avg_price\` | \`Option<String>\`: exact original decimal strings, including quantities far below float precision |
| Prices and trailing orders | \`limit_price\`, \`stop_price\`, \`trail_price\`, \`trail_percent\`, \`hwm\` | \`Option<String>\`, no rounding, no implicit conversion |
| Order semantics | \`order_class\`, \`order_type\`, \`time_in_force\`, \`position_intent\`, \`extended_hours\` | Open typed enums for all but deprecated \`order_type\`; real boolean for \`extended_hours\` |
| Dates | \`created_at\`, \`updated_at\`, \`submitted_at\`, \`filled_at\`, \`canceled_at\`, \`expired_at\`, \`expires_at\`, \`failed_at\`, \`replaced_at\` | \`Option<String>\`, preserving timezone and subsecond spelling |
| Replacements and groups | \`replaced_by\`, \`replaces\`, \`group_id\`, \`group_role\`, \`ratio_qty\` | Optional identifiers/group role and exact decimal ratio (including live option leg extension) |
| Nested legs | \`legs\` | \`Option<Vec<Order>>\`: missing/null versus explicit empty list versus typed children |
| Additional provider metadata | \`source\`, \`subtag\` (not part of published \`Order\` schema) | Optional fields, never inferred or required |

A parent options multi-leg order can have null \`side\`. Both \`qty\` and \`notional\` may be null depending on the order style. Missing and explicit null map to \`None\` inside the typed projection, but are retained separately in raw \`--out\` JSON. A literal \`"0"\` is **not** treated as missing. Wrong JSON scalar types (e.g. numeric \`qty\` or string \`extended_hours\`) are rejected.

### Six open response enum domains

- \`OrderSide\`: \`buy\`, \`sell\`.
- \`OrderState\`: \`new\`, \`partially_filled\`, \`filled\`, \`done_for_day\`, \`canceled\`, \`expired\`, \`replaced\`, \`pending_cancel\`, \`pending_replace\`, \`accepted\`, \`pending_new\`, \`accepted_for_bidding\`, \`stopped\`, \`rejected\`, \`suspended\`, \`calculated\`, \`held\`.
- \`OrderType\`: \`market\`, \`limit\`, \`stop\`, \`stop_limit\`, \`trailing_stop\`.
- \`OrderClass\`: \`simple\`, \`bracket\`, \`oco\`, \`oto\`, \`mleg\`. The provider's legacy empty string \`""\` is accepted as \`simple\`, matching Kirke's current model.
- \`OrderTimeInForce\`: \`day\`, \`gtc\`, \`opg\`, \`cls\`, \`ioc\`, \`fok\`.
- \`OrderPositionIntent\`: \`buy_to_open\`, \`buy_to_close\`, \`sell_to_open\`, \`sell_to_close\`.

Each type retains future unknown string values through \`OpenOrderEnum<Known…>::Other(String)\`, without causing unrelated valid responses to fail. Wrong JSON types are rejected. Known \`""\` order class is the only permitted blank member; other empty enum strings are invalid. The grouping role currently stays optional \`String\`, as in merged Kirke #52; a cross-language enum refinement remains a review decision.

## Nested leg behavior

\`--nested\` is a **request option**: the API decides whether it includes \`legs\`. A present array of leg orders is decoded recursively as typed \`Order\` records. Human summaries include each leg's symbol, side, exact \`ratio_qty\`, and status, prefixed by a \`Legs: N\` count. Like Kirke #52, Medea rejects nonempty **grandchildren** instead of silently losing their structure; missing/null or explicitly empty grandchildren are accepted.

Provider-specific future fields survive in the untouched raw \`--out\` response, but are not fabricated or summarized.

## Published schema requiredness versus operational responses

The current OpenAPI marks top-level \`notional\`, \`type\`, and \`time_in_force\` as required, and six leg properties (\`symbol\`, \`type\`, \`time_in_force\`, \`notional\`, \`qty\`, \`side\`) as required. However, the shared Atalanta/Kirke synthetic contract pack includes quantity-based top-level and leg orders that **omit** \`notional\`, while actual paper responses reliably include \`id\` and \`status\`. For **triparity with merged Kirke #52**, Medea deliberately requires a nonblank \`id\` and typed \`status\`, and treats the class-dependent values as optional. This is a **documented schema deviation**, not a claim that the model enforces every published \`required\` rule. Reconcile strictness against live response variants and Atalanta during the final M3 acceptance review rather than making undocumented, breaking assumptions.

## Tests and integration

The Rust suite uses the same seven **synthetic** fixture files as Kirke #52 (equity, crypto notional, multi-leg options, bracket, trailing stop, unknown future enums, mixed collection) plus a synthetic complete-schema fixture with every published field. It covers typed enum round trips, malformed JSON, nullable/empty/zero values, nested leg summaries, exact fractional quantities and ratios, unrecognized fields, wrong scalar types and recursion limits.

The opt-in \`./scripts/cd.sh\` validates actual read-only Alpaca paper orders with owner-only JSON exports and the same wire types without committing user data. The command displays only selected summary fields; \`--out PATH\` saves original successful provider bytes **atomically with 0600 file mode**.

**Deferred:** order-placement or cancellation functionality (not part of these read-only CLIs), automatic pagination, portfolio aggregation, color, diagnostics, machine-readable stdout, Unix pipe redesign and shell completion. The common presentation contract is tracked in [Medea #44](https://github.com/black-belt-badger/medea/issues/44) / [Kirke #44](https://github.com/black-belt-badger/kirke/issues/44). Keep the parent endpoint issue open until product-owner acceptance and negotiated parity deviations.
