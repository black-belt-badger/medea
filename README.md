<p align="center">
  <img src="docs/fancy.webp" alt="Medea — crucible knot logo and wordmark" width="760">
</p>

# Medea — Rust CLI for Alpaca's Trading and Crypto Market Data APIs

[![Rust CI](https://github.com/black-belt-badger/medea/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/black-belt-badger/medea/actions/workflows/rust-ci.yml)

**Medea** is an unofficial Rust CLI developed alongside [Kirke](https://github.com/black-belt-badger/kirke), its Haskell counterpart. Both aim for the same user-facing Alpaca API behavior while remaining idiomatic in their respective languages.

**Current scope: read-only Alpaca paper account information, open positions, asset discovery, order history/details, and historical crypto bars and trades from Alpaca Market Data.** Medea is not affiliated with, sponsored by, or endorsed by Alpaca Markets.

## Requirements

- Rust **1.99.1**, pinned in `rust-toolchain.toml`, installed through [rustup](https://rustup.rs/).
- Linux or Unix-like system for owner-only JSON output permissions.
- An [Alpaca paper account](https://alpaca.markets/) for authenticated commands. Offline tests do not require one.

## Build, test, and install

```sh
git clone https://github.com/black-belt-badger/medea.git
cd medea
./scripts/ci.sh
medea --help
```

The local script checks formatting, runs Clippy and all tests, builds an optimized executable, then installs or updates `medea` in Cargo's user binary directory (normally `~/.cargo/bin`). Ensure that directory is on your `PATH` to run `medea` from anywhere. Re-run the script after updating the source to install a newer build.

For individual checks:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release
```

GitHub Actions runs the same checks on pull requests and `main` pushes but does not install the CLI globally. The local script does not run the credential-dependent paper API test; use `scripts/cd.sh` explicitly for that.

## API credentials (paper trading and crypto market data)

Medea reads **only** these environment variables:

- `APCA_API_KEY_ID`
- `APCA_API_SECRET_KEY`

The requests use the Alpaca authentication headers over HTTPS to `GET /v2/account`, `GET /v2/positions`, `GET /v2/assets`, `GET /v2/orders`, and `GET /v2/orders/{order_id}` on `https://paper-api.alpaca.markets`. It has a 20-second timeout, does not follow redirects, and never connects to the live trading endpoint.

You can set them directly in your shell, or prepare a private file outside the repository:

```sh
install -d -m 700 "$HOME/.config/medea"
umask 077
${EDITOR:-vi} "$HOME/.config/medea/credentials"
chmod 600 "$HOME/.config/medea/credentials"
```

The file should contain two exports using **paper** account credentials:

```sh
export APCA_API_KEY_ID="YOUR_PAPER_KEY_ID"
export APCA_API_SECRET_KEY="YOUR_PAPER_SECRET_KEY"
```

Source only files you created and trust; do not commit API credentials or paste them into issues, PRs, screenshots, or logs.

```sh
source "$HOME/.config/medea/credentials"
```

This credentials file is for shell convenience; the application itself only reads the environment variables.

## Usage

```sh
cargo run --locked -- account --help
cargo run --locked -- account
```

The first command shows CLI help. The second retrieves a paper account and prints a summary containing account number, status, currency, cash, equity, buying power, trading-blocked flag, and crypto status. Both statuses are typed enums (not just ACTIVE/INACTIVE flags), with unknown future values preserved. Medea now decodes and type-checks all 34 properties in Alpaca's published account response schema: only `id` and `status` are required, and `account_number` can be null. If a summary field is omitted or null, it prints `not provided` rather than inventing a value. See [M3 account API coverage](docs/api/account.md). Example with sensitive values redacted:

```text
Medea — Alpaca paper account
Account number: [redacted]
Status: ACTIVE
Currency: USD
Cash: [redacted]
Equity: [redacted]
Buying power: [redacted]
Trading blocked: no
Crypto status: ACTIVE
```

**The real summary contains your account information and balances; treat it as sensitive.**

To save the complete **unmodified** JSON response instead:

```sh
mkdir -p output
chmod 700 output
cargo run --locked -- account --out output/account.json
```

The destination directory must exist. The file is written through an owner-only temporary file in the same directory and atomically replaces any existing destination. On Unix, the saved JSON has `0600` permissions. `output/` is ignored by Git.

Malformed API responses, rejected credentials, HTTP errors, and file-write failures result in nonzero exit status. Diagnostics never include authentication headers or response bodies.

## Positions: crypto and equities

List **all open positions** in the paper account:

```sh
medea positions
# Or, without a local installation:
cargo run --locked -- positions
```

The command includes crypto and equity positions, showing symbol, typed asset class and long/short side, quantity, market value, current price, and unrealized P/L. The decoder covers **all 22 documented position properties** (17 required), including swap rates and the optional ten-field nested USD object for local-currency accounts. Alpaca supplies financial quantities and amounts as decimal strings; Medea preserves them exactly without floating-point rounding. An empty portfolio prints `No open positions.`. See the [M3 positions coverage matrix](docs/api/positions.md).

Save the original, complete JSON array instead of displaying the summary:

```sh
mkdir -p output
chmod 700 output
medea positions --out output/positions.json
```

The file uses the same private, atomic output procedure as `account --out` (typically `0600` on Unix). The target directory must exist. Both outputs may contain sensitive portfolio data; the ignored `output/` directory is intended for private use.

The endpoint has no symbol filter in this command. **No orders are placed and positions are never closed.** See the [Alpaca positions API reference](https://docs.alpaca.markets/us/reference/getallopenpositions).

## Assets: crypto by default

List crypto assets (explicit default):

```sh
medea assets
```

Alpaca's own default is to list US equities when `asset_class` is omitted; Medea instead sends `asset_class=crypto` explicitly. Choose the class and status:

```sh
medea assets --asset-class us_equity
medea assets --status active
medea assets --asset-class crypto --status inactive
medea assets --status all
```

`--asset-class` supports every documented class: `us_equity`, `us_option`, `crypto` (Medea default), `crypto_perp`, `treasury`, `corporate`, `global_equity`, `us_index`, `us_equity_chain`, `ipo`. `--status` accepts `active`, `inactive`, or `all`; omitted means no status filter.

Additional request filters:

```sh
medea assets --asset-class us_equity --exchange NASDAQ,NYSE --attributes has_options,overnight_tradable
medea assets --asset-class us_index --exchange INDEX
medea assets --asset-class us_equity --attributes ptp_no_exception,ipo --out output/assets.json
```

`--exchange` accepts nine comma-separated values: `AMEX`, `ARCA`, `BATS`, `NYSE`, `NASDAQ`, `NYSEARCA`, `OTC`, `TXSE`, `INDEX`. Use it **only** with `us_equity` (not INDEX) or `us_index` (INDEX only). It is rejected for crypto and other asset classes, rather than being silently ignored.

`--attributes` accepts any of the eight documented values: `ptp_no_exception`, `ptp_with_exception`, `ipo`, `has_options`, `options_late_close`, `fractional_eh_enabled`, `overnight_tradable`, `overnight_halted`. More than one attribute means **any match (OR)**, not all. Empty, duplicate or unknown CSV entries are rejected.

The decoded response covers all 19 documented properties, with typed class/exchange/status/attribute enums, required fields, nullable metadata and exact decimal strings. The concise summary continues to show symbol, class, status, name, tradability and fractionability. It prints `No matching assets.` for an empty array. See [M3 Assets API coverage](docs/api/assets.md).

Save the full original JSON array with private, atomic file replacement:

```sh
mkdir -p output
chmod 700 output
medea assets --asset-class crypto --status active --out output/assets.json
```

On Unix, the saved file has `0600` permissions. The directory must exist. Unknown future provider fields remain preserved in the output file. This command makes no state-changing or trading requests.

See [Alpaca's GET /v2/assets reference](https://docs.alpaca.markets/us/reference/get-v2-assets-1).

## Orders: read-only history and filters

List open orders, newest first, or specify filters:

```sh
medea orders
medea orders --status all --limit 10
medea orders --status closed --direction asc
medea orders --symbols BTCUSD,ETHUSD,AAPL --limit 50
```

- `--status open|closed|all` defaults to `open`; `--direction asc|desc` defaults to `desc`.
- `--limit 1..500` is optional (Alpaca's default is 50 orders). `--symbols` takes comma-separated trading symbols such as `BTCUSD`, not `BTC/USD`.
- Values are validated before networking. Prices and quantities are preserved as **strings**, never converted to floating point. Nullable fields print `not provided`.
- An empty response prints `No matching orders.`. This endpoint lists a single page; it does not automatically paginate.

To save the **complete original JSON response** instead of a terminal summary:

```sh
mkdir -p output && chmod 700 output
medea orders --status all --limit 10 --out output/orders.json
```

The file is atomically replaced with owner-only permissions (normally `0600` on Unix). Order activity is sensitive. **This command cannot place, modify or cancel orders** and never uses the live trading endpoint. See [Alpaca GET /v2/orders](https://docs.alpaca.markets/us/reference/getallorders-1). Medea and [Kirke](https://github.com/black-belt-badger/kirke) intentionally expose matching CLI filters and readable fields.

## Order details: read-only lookup by server UUID

Look up an individual paper order using its server-issued `id` (not its `client_order_id`):

```sh
medea order get 9275f66a-0a1b-4a22-8c55-167416c7cf14
medea order get 9275f66a-0a1b-4a22-8c55-167416c7cf14 --nested
medea order get 9275f66a-0a1b-4a22-8c55-167416c7cf14 --out output/order.json
```

The UUID above is an **illustrative placeholder**, not a real order. Use an ID from your paper account's `medea orders --out` response. `medea order --help` and `medea order get --help` show the available options.

Medea requires a canonical 36-character UUID and rejects malformed IDs, path separators, query strings and fragments *before* loading credentials. `--nested` sends the optional Alpaca `nested=true` query parameter to roll up multi-leg orders; without it no query string is sent. A missing order produces a dedicated `Alpaca paper order not found (HTTP 404)` message, distinct from authentication, transport or decoding failures.

A successful lookup returns a **JSON object**, not an array. The readable view reuses the order-listing fields and preserves decimal strings and nullability. `--out PATH` saves the **complete original JSON**, including extra fields or nested `legs`, by atomic replacement with owner-only permissions (normally `0600`). The parent directory must exist.

Like Kirke, this command is **read-only and paper-only**. It does not place, modify or cancel orders. Official documentation: [GET /v2/orders/{order_id}](https://docs.alpaca.markets/us/reference/getorderbyorderid-1).

## Historical crypto bars (Alpaca Market Data)

Historical bars use **`https://data.alpaca.markets/v1beta3/crypto/{loc}/bars`**, authenticated with the same API-key variables. This is a **read-only market-data endpoint**, separate from paper trading. The command never submits orders.

```sh
medea crypto bars \
  --symbols BTC/USD,ETH/USD,SOL/USD --timeframe 1Hour \
  --start 2026-10-02T00:00:00Z --end 2026-10-03T00:00:00Z

medea crypto bars \
  --symbols BTC/USD,ETH/USD --timeframe 5Min \
  --start 2026-10-02 --end 2026-10-03 \
  --limit 100 --max-pages 3 --sort asc --out output/crypto-bars.json

medea crypto bars \
  --symbols SOL/USD --timeframe 1Min --limit 100 --max-pages 2 \
  --format jsonl --out output/sol-bars.jsonl
```

**Required:** `--symbols` accepts 1–30 comma-separated pairs in the form `BTC/USD` (unlike order filtering, which uses `BTCUSD`). `--timeframe` supports 1–59 minutes, 1–23 hours, 1 day, 1 week, and 1/2/3/4/6/12 months, including Alpaca's short aliases. Optional settings: `--location us|us-1|us-2|eu-1|bs-1` (default `us`), inclusive `--start`/`--end` (YYYY-MM-DD or timezone-qualified RFC3339), `--limit 1..10000` (default 1000 **across all symbols**), `--sort asc|desc` (default `asc`), `--page-token` to resume, and `--max-pages 1..20` (default **1**). Alpaca can return less than the page limit while more pages remain, so inspect the `next_page_token`. See [Alpaca's historical bars reference](https://docs.alpaca.markets/us/reference/cryptobars-1).

**Output contract:** Without `--out`, default `--format json` prints a short per-symbol summary and whether more pages remain. With `--out PATH`, it writes `{"pages":[API_PAGE_1,API_PAGE_2,...],"next_page_token":...}`, retaining complete provider pages and unknown fields. A non-null final token permits a subsequent request with `--page-token`. With `--format jsonl`, stdout or `--out PATH` contains one JSON record per bar with `symbol`, `t`, `o`, `h`, `l`, `c`, `v`, `n`, and `vw`. JSONL does not retain a continuation token; select JSON export if resumption matters.

Rust uses **`serde_json::Number` with arbitrary precision** for the numeric OHLCV/VWAP fields, never binary floating point. Fractional nanosecond timestamps are kept verbatim. Number formatting may be normalized (for example, scientific notation), but the **numeric value is not rounded**. JSON page envelopes preserve extra upstream fields, while JSONL contains the known bar fields. Requests are capped at 20 pages of up to 10,000 bars each; memory use is explicitly bounded, without a bulk-backfill planner. A repeated page token or malformed response aborts without writing a partial file. Exports are private and atomically replaced, normally `0600` on Unix; their parent directory must exist.

This is a data-access CLI, not a dataset inventory or backfill planner (those belong to Metis/Ariadne). No live-trading or state-changing operations are performed.

## Historical crypto trades (Alpaca Market Data)

Individual executions are fetched using **`GET https://data.alpaca.markets/v1beta3/crypto/{loc}/trades`**. It uses the same authenticated, read-only HTTPS market-data client as bars. **Trades do not have a `--timeframe` argument.**

```sh
medea crypto trades \
  --symbols BTC/USD,ETH/USD,SOL/USD \
  --start 2026-10-02T00:00:00Z --end 2026-10-03T00:00:00Z

medea crypto trades \
  --symbols BTC/USD,ETH/USD --location us --limit 100 \
  --max-pages 3 --sort asc --out output/crypto-trades.json

medea crypto trades \
  --symbols BTC/USD --limit 1000 --max-pages 2 \
  --format jsonl --out output/btc-trades.jsonl
```

**Filters:** `--symbols` (required, 1–30 comma-separated pairs such as `BTC/USD`), `--location us|us-1|us-2|eu-1|bs-1` (default `us`), inclusive `--start`/`--end` (YYYY-MM-DD or timezone-qualified RFC3339), `--limit 1..10000` (default 1000 **across all symbols**), `--sort asc|desc` (default `asc`), `--max-pages 1..20` (default **1**), and optional `--page-token` to continue from a previous JSON export. A short result page does not imply completion: inspect `next_page_token`. See [Alpaca historical trades reference](https://docs.alpaca.markets/us/reference/cryptotrades-1).

**Output:** Without `--out`, the default `--format json` prints page and per-symbol trade counts and indicates if more pages remain. With `--out PATH`, it writes `{"pages":[API_PAGE_1,API_PAGE_2,...],"next_page_token":...}`: complete, original upstream JSON page objects and the last continuation token. Resume by passing `--page-token`. With `--format jsonl`, stdout or `--out PATH` contains a record per trade with `symbol`, `i` (integer ID), `p` (price), `s` (size), `t` (timestamp), `tks` (taker side or null). JSONL does **not** retain the continuation token; use JSON for resumption.

Rust stores trade IDs as arbitrary-precision integer JSON numbers and uses **`serde_json::Number` with `arbitrary_precision`** for price and size, without binary floating-point rounding. Timestamp strings retain their nanosecond precision, and absent/null taker sides are supported. Complete upstream JSON pages retain additional provider fields, though whitespace and numeric spelling may be normalized when reserialized.

**Memory and safety:** Each page is decoded and written before the next one is fetched; the whole trade history is **not retained in memory**. A private temporary file in the destination directory is atomically renamed only on success (mode normally `0600`), preserving any existing destination after HTTP, decoding or output failures. JSONL sent to **stdout** may have printed earlier records before a later request fails; use `--out` for reliable all-or-nothing files. The parent directory must already exist. Bounded pagination rejects repeated tokens instead of looping indefinitely.

This is bounded historical data access, not inventory or backfill planning (those belong to Metis/Ariadne). No orders are submitted, cancelled or modified.

## Integration check (maintainers)

The explicit script `scripts/cd.sh` runs the real **paper-only** account, positions, assets, and order commands, plus bounded historical crypto-bars and crypto-trades requests, checks JSON structures and private output permissions, and leaves any account data in the ignored `output/` directory. It is **not** run in public CI.

On a trusted Linux machine with `jq`, `stat`, and paper credentials:

```sh
./scripts/cd.sh
```

By default the script sources `${XDG_CONFIG_HOME:-$HOME/.config}/medea/credentials`. If your private credentials live elsewhere, pass their path via `MEDEA_CREDENTIALS_FILE`; for example, maintainers sharing Kirke's paper credentials can use:

```sh
MEDEA_CREDENTIALS_FILE="$HOME/.config/kirke/credentials" ./scripts/cd.sh
```

Do not paste the script's private outputs into CI, an issue, or a release.

## Visual identity

<img src="docs/color.png" alt="Medea — small color icon" width="40" height="40">

The logo, illustrations, and source images are documented in [Branding and asset usage](docs/BRANDING.md). They are project-specific, AI-assisted artwork selected by the maintainer, **not** official Alpaca artwork or a mark of affiliation. The README banner uses an optimized WebP display copy while the original images remain committed.

## Roadmap, contributions, and safety

Next: historical crypto quotes, account state, and carefully controlled paper trading. **Live trading, order submission/cancellation, and streaming are not part of this version.**

Medea is available under the [BSD 3-Clause License](LICENSE), copyright © 2026 Marek Dudek. See the [GitHub Releases](https://github.com/black-belt-badger/medea/releases) page for source releases and version notes.

See [GitHub Issues](https://github.com/black-belt-badger/medea/issues), [CONTRIBUTING.md](CONTRIBUTING.md), [SECURITY.md](SECURITY.md) for private vulnerability reporting, and [RELEASING.md](RELEASING.md) for the release procedure. Release milestones track feature parity and publication checks.
