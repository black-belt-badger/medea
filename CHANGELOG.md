# Changelog for `medea`

All notable changes to this project will be documented here.

Versions use [Semantic Versioning](https://semver.org/).

## Unreleased

### Changed
- Model `account.status` and optional `account.crypto_status` as the same forward-compatible `AccountStatus` enum; unknown future values survive parsing without changing human summaries or raw provider JSON exports.
- Decode all 34 documented `GET /v2/account` response fields with strict wire types, including two integer options-level enums. Only `id` and `status` are required; absent/null optional values display as `not provided`, while `--out` still preserves the complete original provider JSON.
- Complete `GET /v2/assets` API fidelity: ten asset classes, typed `--exchange` and `--attributes` filters, equity/index restrictions, all 19 documented response fields, forward-compatible enum decoding and exact financial numbers. Preserve the crypto CLI default and private raw JSON exports.
- Complete `GET /v2/positions` decoding: 22 documented response fields (17 required), nested ten-field USD values for local-currency accounts, shared typed asset class/exchange enums, typed long/short side with unknown-value fallback and exact decimal strings. Preserve current summaries and private original JSON exports.
- Expand read-only `GET /v2/orders` request fidelity to all 11 documented query filters: typed side and asset classes, exact timezone-qualified submission bounds, nested multi-leg grouping and mutually exclusive order-UUID cursors. Preserve existing safe defaults, output formats, and private JSON; defer complete response decoding to the next phase.
- Complete `GET /v2/orders` and single-order response fidelity: all 37 top-level and 35 nested-leg published fields, six open typed enum domains, exact decimal strings, typed bracket/multi-leg children, leg ratio summaries and shared synthetic Kirke/Atalanta fixtures. Continue preserving original JSON bytes and documenting provider schema requiredness differences.
- Complete `GET /v2/orders/{order_id}` fidelity: verify returned order identity matches the requested UUID before rendering or exporting; distinguish nonexistence (404), authentication, HTTP and transport failures and sanitize malformed/mismatched responses. Preserve exact provider bytes, typed nested legs and private output; align with Kirke while recording the Atalanta optional-404 difference.

## [0.2.0] - 2026-11-13

### Added
- `medea crypto trades` fetches bounded historical individual crypto executions with validated symbols, locations, date bounds and continuation tokens, reusing the authenticated read-only market-data request builder.
- Precise numeric trade IDs/prices/sizes, nanosecond timestamp strings, optional taker sides, incremental JSON/JSONL output and atomic private rollback on partial failures; offline and live integration tests.
- `medea crypto bars` retrieves historical crypto OHLCV from Alpaca Market Data with typed, validated timeframes, locations, date bounds, sort and bounded next-token pagination.
- Exact decimal-number decoding and nanosecond timestamp strings, complete JSON page-envelope export, JSONL records, private output, offline tests and a real market-data integration check.
- `medea order get ORDER_ID` retrieves an individual paper order by validated UUID, with optional `--nested` and private `--out PATH` for full provider JSON.
- Reuses the existing typed order decoder and summary fields; separate HTTP 404 handling, offline tests and a paper API integration check.
- `medea orders` lists read-only paper orders with typed status, limit, symbols and direction filters; financial values retain exact decimal strings.
- `medea orders --out PATH` saves complete order JSON privately and atomically, with deterministic offline tests and explicit paper integration.
- `medea assets` lists crypto assets explicitly by default, with typed asset-class and status filters.
- `medea assets --out PATH` preserves complete assets JSON privately; offline and real paper API tests cover query construction, JSON decoding, and empty results.
- `medea positions` lists all open Alpaca paper account positions for crypto and equities, with exact decimal strings and readable empty-portfolio handling.
- `medea positions --out PATH` saves the original JSON array atomically with owner-only permissions.
- Offline tests for typed position decoding, missing and null values, request security, CLI parsing and output.

### Changed
- Share authenticated paper-only HTTP requests and private JSON output between account and positions commands.

## [0.1.0] - 2026-11-12

### Added
- `medea account` fetches a paper account and prints a readable summary.
- `medea account --out PATH` saves unmodified account JSON with owner-only permissions and atomic replacement.
- Credential validation, strict JSON decoding with exact monetary strings, paper-only HTTPS with timeout and redirects disabled.
- Offline Rust unit and CLI tests, Rust CI (format, Clippy, tests, release build), and an optional private paper API integration check.
- Public README, contribution templates, security reporting policy, release procedure, and BSD-3-Clause licensing.
