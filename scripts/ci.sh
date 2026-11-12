#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release

# Install/update the CLI on this machine. Reuse the release build above;
# the project's Cargo.lock keeps the dependency set reproducible.
cargo install --path . --locked --force --target-dir target
