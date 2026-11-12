# Releasing Medea (maintainers)

Medea is an **unofficial** Rust client and CLI for Alpaca's APIs. The repository is currently private, and the first release should wait until the paper account feature works.

## Versioning

- Use [Semantic Versioning](https://semver.org/) with three components: `MAJOR.MINOR.PATCH`.
- Keep `Cargo.toml` and Git tags aligned: package `0.1.0` -> tag `v0.1.0`.
- Before `1.0.0`, breaking changes can occur, but document them in `CHANGELOG.md`.
- Releases are independent from Haskell [Alpaca](https://github.com/black-belt-badger/alpaca), which uses four-component Haskell PVP versions.
- Track the first release in its GitHub milestone and in shared Project #7.

## Release checklist

1. Complete the release milestone, especially **account feature parity**, a tested CLI, a clear README, an explicit license and confirmed copyright/maintainer attribution.
2. Update `CHANGELOG.md` with a dated `0.1.0` heading and update `Cargo.toml` to the intended version.
3. Confirm the owner has approved release/public visibility separately. **A GitHub release does not imply that the private repository should become public.**
4. From a clean working tree, verify:

   ```sh
   cargo fmt --all -- --check
   cargo build --release
   cargo test
   ```

5. Manually exercise the Alpaca **paper-only** account command on a trusted machine with private environment-based credentials; record success without publishing secrets or real account data. Once Rust CI exists, require its passing check before tagging.
6. Review release notes for actual behavior, limitations, supported platforms, and the unofficial status of this implementation.

## Create a tag and GitHub Release

After all of the above has been verified and merged:

```sh
git switch main
git pull --ff-only origin main
git status --short
git tag -a v0.1.0 -m "Medea v0.1.0"
git push origin v0.1.0

gh release create v0.1.0 \
  --repo black-belt-badger/medea \
  --verify-tag \
  --title "Medea v0.1.0" \
  --generate-notes \
  --prerelease
```

These are **examples for a future release**, not commands to run before the implementation and license are ready. Review the generated release notes; GitHub provides source archives by default.

Afterward, verify the tag and release and close the milestone. Never force-rewrite an already published tag.
