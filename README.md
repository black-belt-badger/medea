# Medea — Rust CLI for Alpaca's paper Trading API

[![Rust CI](https://github.com/black-belt-badger/medea/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/black-belt-badger/medea/actions/workflows/rust-ci.yml)

**Medea** is an unofficial Rust CLI developed alongside [Kirke](https://github.com/black-belt-badger/kirke), its Haskell counterpart. Both aim for the same user-facing Alpaca API behavior while remaining idiomatic in their respective languages.

**Current scope: read-only Alpaca paper account access.** Medea is not affiliated with, sponsored by, or endorsed by Alpaca Markets.

## Requirements

- Rust **1.99.1**, pinned in `rust-toolchain.toml`, installed through [rustup](https://rustup.rs/).
- Linux or Unix-like system for owner-only JSON output permissions.
- An [Alpaca paper account](https://alpaca.markets/) for authenticated commands. Offline tests do not require one.

## Build and test

```sh
git clone https://github.com/black-belt-badger/medea.git
cd medea
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release
./target/release/medea --help
```

The same checks run in GitHub Actions on pull requests and `main` pushes.

## Paper credentials

Medea reads **only** these environment variables:

- `APCA_API_KEY_ID`
- `APCA_API_SECRET_KEY`

The request uses HTTPS `GET https://paper-api.alpaca.markets/v2/account` with the Alpaca authentication headers. It has a 20-second timeout, does not follow redirects, and never connects to the live trading endpoint.

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

## Paper integration check (maintainers)

The explicit script `scripts/it.sh` runs the real **paper-only** account command, checks the JSON fields and private output permissions, and leaves any account data in the ignored `output/` directory. It is **not** run in public CI.

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

Next: account state, historical crypto market data, and carefully controlled paper trading. **Live trading, order placement, streaming, and market-data commands are not part of this version.**

See [GitHub Issues](https://github.com/black-belt-badger/medea/issues), [CONTRIBUTING.md](CONTRIBUTING.md), and [RELEASING.md](RELEASING.md). The first `v0.1.0` release awaits final documentation, license metadata, and review under [issue #4](https://github.com/black-belt-badger/medea/issues/4).
