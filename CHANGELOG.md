# Changelog for `medea`

All notable changes to this project will be documented here.

Versions use [Semantic Versioning](https://semver.org/).

## Unreleased

No changes yet.

## [0.1.0] - 2026-11-12

### Added
- `medea account` fetches a paper account and prints a readable summary.
- `medea account --out PATH` saves unmodified account JSON with owner-only permissions and atomic replacement.
- Credential validation, strict JSON decoding with exact monetary strings, paper-only HTTPS with timeout and redirects disabled.
- Offline Rust unit and CLI tests, Rust CI (format, Clippy, tests, release build), and an optional private paper API integration check.
- Public README, contribution templates, security reporting policy, release procedure, and BSD-3-Clause licensing.
