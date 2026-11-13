# Releasing Medea (maintainers)

Medea is an **unofficial Rust CLI** for read-only paper Trading API state and historical crypto market data. Versions follow [Semantic Versioning](https://semver.org/) and are independent of Haskell [Kirke](https://github.com/black-belt-badger/kirke).

## Versioning

- Keep `Cargo.toml`, `Cargo.lock` and the tag aligned (package `0.2.0` → tag `v0.2.0`).
- Releases prior to `1.0.0` may evolve compatibility; record changes in `CHANGELOG.md`.
- Do not force-update published release tags. Make a new version for corrections.

## Release readiness

1. Ensure the owner's approval to publish, all release-preparation changes are reviewed and merged, and supported operations and limitations are accurate. M2 Account and Trading State is closed; this release also includes M4 historical bars/trades, while **historical quotes remain out of scope**.
2. Update the package version, lockfile, dated `CHANGELOG.md` and release notes at `docs/releases/v0.2.0.md`. Never replace confirmed license/author attribution with guesses.
3. Verify the exact `main` commit has a successful required **Rust CI (1.99.1)** GitHub Actions check.
4. Run the full local checks on a clean source tree:

   ```sh
   cargo fmt --all -- --check
   cargo clippy --locked --all-targets --all-features -- -D warnings
   cargo test --locked --all-targets
   cargo build --locked --release
   ./scripts/cd.sh
   ```

   The opt-in integration script tests paper-account/order reads and bounded historical crypto bars/trades using private local credentials. **Never upload actual account data, crypto responses or secrets** to public CI or release assets.
5. Confirm docs, unofficial status, copyright/license, supported Rust 1.99.1 toolchain, file permissions and lack of any live-trading/write operations. GitHub publishes source archives; no compiled binaries or crates.io package are provided.

## Tag and publish after the release-preparation PR is merged

```sh
git switch main
git pull --ff-only origin main
git status --short
git tag -a v0.2.0 -m "Medea v0.2.0"
git push origin v0.2.0

gh release create v0.2.0 \
  --repo black-belt-badger/medea \
  --verify-tag \
  --title "Medea v0.2.0" \
  --notes-file docs/releases/v0.2.0.md \
  --prerelease
```

Only tag the **tested, post-merge `main` commit**. Review the created release and verify its notes and source archive. Do not rewrite already published tags.
