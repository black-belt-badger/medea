# Historical crypto quotes — Medea CLI

**Endpoint:** authenticated, read-only `GET https://data.alpaca.markets/v1beta3/crypto/{loc}/quotes`. This is **market data**, not a Trading API order endpoint. The command never changes account state.

## Examples

```sh
# Compact per-symbol summary (one page by default)
medea crypto quotes --symbols BTC/USD,ETH/USD --location us \
    --start 2026-10-02T00:00:00Z --end 2026-10-03T00:00:00Z

# Bounded multi-page JSON export with original provider envelopes
medea crypto quotes --symbols BTC/USD,SOL/USD --limit 500 \
    --max-pages 3 --sort asc --out output/quotes.json

# Incremental JSONL output, one symbolized quote per line
medea crypto quotes --symbols BTC/USD --format jsonl \
    --max-pages 2 --out output/btc-quotes.jsonl

# Resume an incomplete JSON export using its next_page_token
medea crypto quotes --symbols BTC/USD --page-token 'TOKEN_FROM_PREVIOUS_JSON' \
    --max-pages 2 --out output/quotes-next.json
```

## Request contract

| Alpaca parameter | CLI | Behavior |
| --- | --- | --- |
| `loc` | `--location` | `us` (default), `us-1`, `us-2`, `eu-1`, `bs-1`. Typed location appears in URL path; never untrusted path interpolation |
| `symbols` | `--symbols` | Required: 1–30 comma-separated `BASE/QUOTE` pairs, such as `BTC/USD,ETH/USD`, strictly validated |
| `start` | `--start` | Optional, inclusive YYYY-MM-DD or RFC3339; preserve original qualified timezone and fractional timestamp |
| `end` | `--end` | Optional, inclusive YYYY-MM-DD or RFC3339; reject reversed absolute instants before credentials/network |
| `limit` | `--limit` | 1–10,000, default **1,000 per response page total**, not per symbol |
| `sort` | `--sort asc\|desc` | Default ascending |
| `page_token` | `--page-token` | Opaque provider continuation, URL-encoded once; do not fabricate or parse it |
| Local cap | `--max-pages` | 1–20 pages, default **1**; reports continuation if bounded response is incomplete |
| Output | `--format json\|jsonl`, `--out PATH` | Default JSON format means a concise console summary without `--out`; with `--out`, complete provider pages. JSONL emits symbolized records |

**No `--timeframe`:** quotes are per-timestamp bid/ask observations, unlike aggregated crypto bars. The date interval is inclusive. The API can return a short page even when more data exists, so terminate only when `next_page_token` is null/empty or the explicit page cap is reached. The current Alpaca reference lists all five locations above; venue availability and earliest historical dates vary. The `us-1` feed is documented from **2025-10-14** onward.

## Response / precision

```json
{
  "quotes": {
    "BTC/USD": [
      {
        "ap": 29059,
        "as": 3.252,
        "bp": 29058,
        "bs": 0.3544,
        "t": "2022-05-26T11:47:18.44347136Z"
      }
    ]
  },
  "next_page_token": null
}
```

| Wire property | Rust type | Meaning |
| --- | --- | --- |
| `quotes` | `BTreeMap<String, Vec<CryptoQuote>>` | Symbol-keyed quote arrays; empty symbols/results allowed |
| `ap`, `as` | `serde_json::Number` | Ask price, ask size |
| `bp`, `bs` | `serde_json::Number` | Bid price, bid size |
| `t` | `String` | Exact timestamp text, including nanosecond fractions |
| `next_page_token` | `Option<String>` | Opaque resume token; null/missing/empty means end |
| Unknown provider keys | `CryptoQuotesPage.original: Value` | Full page envelope retains future fields and precise JSON values |

`serde_json` enables `arbitrary_precision`: no quote prices or sizes are converted through binary `f64`. Decimal notation may be normalized on output, but the precise numeric value is preserved. Missing, null and wrongly typed required quote fields are invalid responses.

With `--out PATH --format json`, the output is:

```json
{"pages":[{"quotes":{"BTC/USD":[]},"next_page_token":"..."}],"next_page_token":"..."}
```

Each `pages` item is the complete provider object, including undocumented fields. The envelope's trailing token comes from the last fetched page. With `--format jsonl`, each line has `symbol`, `ap`, `as`, `bp`, `bs` and `t`; unknown per-quote extension fields and continuation tokens are *not* in JSONL. Use full JSON export when raw extensions or resume metadata matter.

## Bounded streaming and safety

- Read-only HTTPS transport shared with bars/trades, identical credential headers, 20s timeout, and redirects disabled.
- Decode and write **one page at a time**. The CLI does not keep a growing buffer of all quotes.
- Detect a repeated continuation token and stop instead of looping indefinitely.
- Private output uses an owner-only temporary file in the destination directory, atomically renamed after all selected pages succeed. A malformed later page, 429, timeout or writer failure leaves an existing destination untouched. The directory must already exist.
- JSONL to **stdout** is streamed immediately; records already printed cannot be rolled back if a later page fails. Use `--out` for all-or-nothing output.
- Auth, rate limit, server, transport, invalid JSON and invalid CLI arguments receive sanitized errors. Credentials and provider error bodies are never copied into public CI.

## Offline tests / scope

Unit tests cover source fields, exact long decimals, high-precision timestamps, future keys, wrong/missing types, empty arrays, pagination and repeated tokens, HTTP error classification, safe request query encoding, private atomic rollback, and output formatting. CLI integration tests check help text, limits, symbols, timestamp validation, missing credentials and absence of a fictitious timeframe. **No live calls or private data in automated tests.**

The implementation follows Medea's existing crypto bars/trades conventions. Further changes to global CLI presentation, exit codes, colors or piping remain **deferred until the architecture/triparity review**.

## References

- [Alpaca historical crypto quotes](https://docs.alpaca.markets/us/reference/cryptoquotes-1)
- [Medea endpoint issue #13](https://github.com/black-belt-badger/medea/issues/13)
- [Kirke paired quotes issue #19](https://github.com/black-belt-badger/kirke/issues/19)
