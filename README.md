# Medea — Rust CLI for Alpaca's Trading API

**Medea** is an unofficial Rust command-line application being developed alongside [Kirke](https://github.com/black-belt-badger/kirke), its Haskell counterpart. We aim to implement the same outward Alpaca API behavior in both languages while keeping each implementation idiomatic.

**Status: bootstrap only.** The current program simply prints `Medea`. The paper-account command, HTTP client, API authentication, and trading operations are **not yet implemented**.

Medea is not affiliated with, sponsored by, or endorsed by Alpaca Markets.

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) via `rustup`, including Cargo.
- Rust **1.99.1**, pinned in `rust-toolchain.toml`; `rustup` selects it automatically.
- No Alpaca credentials are needed for the current placeholder program or offline checks.

## Build and run

```sh
git clone https://github.com/black-belt-badger/medea.git
cd medea
cargo run --locked
```

At present, the output is:

```text
Medea
```

To run the same local checks as GitHub Actions:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release
```

GitHub Actions executes these on pull requests and on pushes to `main`. The repository remains private during initial development.

## Roadmap

The next feature is a **read-only Alpaca paper-account command** matching Kirke's behavior ([issue #3](https://github.com/black-belt-badger/medea/issues/3)). Following work covers account state, historical crypto market data, and carefully controlled paper trading. No live trading or real order placement is currently supported.

The initial `v0.1.0` milestone tracks account parity, documentation, release metadata, and safety checks. We do not publish a release until its functionality has been implemented and verified.

## Contributions and security

See [CONTRIBUTING.md](CONTRIBUTING.md) for development conventions and [RELEASING.md](RELEASING.md) for the release checklist.

Never commit Alpaca API keys, secret tokens, or real account data. Future API requests will use **paper** credentials, provided outside version control. Report security-sensitive problems privately rather than in public issues.

Licensing and final public release metadata are being tracked in [issue #4](https://github.com/black-belt-badger/medea/issues/4).
