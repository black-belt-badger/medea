# Changelog for `medea`

All notable changes to this project will be documented here.

Versions use [Semantic Versioning](https://semver.org/).

## Unreleased

### Changed
- Model `account.status` and optional `account.crypto_status` as the same forward-compatible `AccountStatus` enum; unknown future values survive parsing without changing human summaries or raw provider JSON exports.

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
