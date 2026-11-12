<p align="center">
  <img src="docs/fancy.png" alt="Medea — crucible knot logo and wordmark" width="760">
</p>

# Medea — Rust CLI for Alpaca's paper Trading API

[![Rust CI](https://github.com/black-belt-badger/medea/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/black-belt-badger/medea/actions/workflows/rust-ci.yml)

**Medea** is an unofficial Rust CLI developed alongside [Kirke](https://github.com/black-belt-badger/kirke), its Haskell counterpart. Both aim for the same user-facing Alpaca API behavior while remaining idiomatic in their respective languages.

**Current scope: read-only Alpaca paper account information, open positions, asset discovery, and order history/details.** Medea is not affiliated with, sponsored by, or endorsed by Alpaca Markets.

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

GitHub Actions runs the same checks on pull requests and `main` pushes but does not install the CLI globally. The local script does not run the credential-dependent paper API test; use `scripts/it.sh` explicitly for that.

## Paper credentials

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

The first command shows CLI help. The second retrieves a paper account and prints a summary containing account number, status, currency, cash, equity, buying power, trading-blocked flag, and optional crypto status. Example with sensitive values redacted:

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

The command includes crypto and equity positions, showing symbol, asset class, long/short side, quantity, market value, current price, and unrealized P/L. Alpaca supplies financial quantities and amounts as decimal strings; Medea preserves them exactly, without floating-point rounding. Absent values print as `not provided`. An empty portfolio prints `No open positions.`

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

`--asset-class` accepts `crypto` or `us_equity`. `--status` accepts `active`, `inactive`, or `all`; the omitted status means no filter. The command displays the symbol, class, status, name, tradability, and fractional-trading eligibility when provided. It prints `No matching assets.` for an empty response.

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

## Paper integration check (maintainers)

The explicit script `scripts/it.sh` runs the real **paper-only** account, positions, assets, order-listing, and single-order commands, checks JSON structures and private output permissions, and leaves any account data in the ignored `output/` directory. It is **not** run in public CI.

On a trusted Linux machine with `jq`, `stat`, and paper credentials:

```sh
./scripts/it.sh
```

By default the script sources `${XDG_CONFIG_HOME:-$HOME/.config}/medea/credentials`. If your private credentials live elsewhere, pass their path via `MEDEA_CREDENTIALS_FILE`; for example, maintainers sharing Kirke's paper credentials can use:

```sh
MEDEA_CREDENTIALS_FILE="$HOME/.config/kirke/credentials" ./scripts/it.sh
```

Do not paste the script's private outputs into CI, an issue, or a release.

## Roadmap, contributions, and safety

Next: account state, historical crypto market data, and carefully controlled paper trading. **Live trading, order submission/cancellation, streaming, and market-data commands are not part of this version.**

Medea is available under the [BSD 3-Clause License](LICENSE), copyright © 2026 Marek Dudek. See the [GitHub Releases](https://github.com/black-belt-badger/medea/releases) page for source releases and version notes.

See [GitHub Issues](https://github.com/black-belt-badger/medea/issues), [CONTRIBUTING.md](CONTRIBUTING.md), [SECURITY.md](SECURITY.md) for private vulnerability reporting, and [RELEASING.md](RELEASING.md) for the release procedure. Release milestones track feature parity and publication checks.
