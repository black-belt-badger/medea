#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

umask 077
mkdir -p output
chmod 700 output

# Only source a trusted paper-credentials file, never checked into Git.
# shellcheck source=/dev/null
source "${MEDEA_CREDENTIALS_FILE:-${XDG_CONFIG_HOME:-$HOME/.config}/medea/credentials}"

cargo build --locked --release --quiet
./target/release/medea --help > /dev/null
./target/release/medea account --help > /dev/null
./target/release/medea account > ./output/account-summary.txt
./target/release/medea account --out ./output/account.json > ./output/account-written.txt
./target/release/medea positions --help > /dev/null
./target/release/medea positions > ./output/positions-summary.txt
./target/release/medea positions --out ./output/positions.json > ./output/positions-written.txt
./target/release/medea assets --help > /dev/null
./target/release/medea assets --status active > ./output/assets-summary.txt
./target/release/medea assets --status active --out ./output/assets.json > ./output/assets-written.txt
# Exercise M3 request filters together using read-only paper data.
./target/release/medea assets --asset-class us_equity --exchange NASDAQ --attributes has_options \
  --out ./output/assets-nasdaq-options.json > ./output/assets-nasdaq-options-written.txt
./target/release/medea orders --help > /dev/null
./target/release/medea orders --status all --limit 10 > ./output/orders-summary.txt
./target/release/medea orders --status all --limit 10 --out ./output/orders.json > ./output/orders-written.txt
# M3: exercise combined request filters against the read-only paper API.
./target/release/medea orders --status all --limit 10 --side buy \
  --asset-class crypto --nested \
  --out ./output/orders-filtered.json > ./output/orders-filtered-written.txt

test -s ./output/account-summary.txt
grep -q '^Medea — Alpaca paper account$' ./output/account-summary.txt
test -s ./output/account.json
jq -e '.id and .status and .cash and .equity and .buying_power' ./output/account.json > /dev/null
test "$(stat -c '%a' ./output/account.json)" = 600
test "$(stat -c '%a' ./output/account-summary.txt)" = 600

test -s ./output/positions-summary.txt
grep -q '^Medea — Alpaca paper positions$' ./output/positions-summary.txt
jq -e 'type == "array"' ./output/positions.json > /dev/null
# Verify the actual paper API honors the required M3 positions wire types.
jq -e 'type == "array" and all(.[];
  (.asset_id | type) == "string" and
  (.asset_class | type) == "string" and
  (.exchange | type) == "string" and
  (.side | type) == "string" and
  (.qty | type) == "string" and
  (.market_value | type) == "string" and
  (.current_price | type) == "string" and
  (.asset_marginable | type) == "boolean")' ./output/positions.json > /dev/null
test "$(stat -c '%a' ./output/positions.json)" = 600
test "$(stat -c '%a' ./output/positions-summary.txt)" = 600

test -s ./output/assets-summary.txt
grep -q '^Medea — Alpaca paper assets$' ./output/assets-summary.txt
jq -e 'type == "array" and all(.[]; .class == "crypto" and (.tradable | type == "boolean"))' ./output/assets.json > /dev/null
test "$(stat -c '%a' ./output/assets.json)" = 600
test "$(stat -c '%a' ./output/assets-summary.txt)" = 600
jq -e 'type == "array" and all(.[]; .class == "us_equity" and .exchange == "NASDAQ" and ((.attributes // []) | index("has_options") != null))' ./output/assets-nasdaq-options.json > /dev/null
test "$(stat -c '%a' ./output/assets-nasdaq-options.json)" = 600

test -s ./output/orders-summary.txt
grep -q '^Medea — Alpaca paper orders$' ./output/orders-summary.txt
jq -e 'type == "array" and (length <= 10) and all(.[]; .id and .client_order_id and .symbol and .status)' ./output/orders.json > /dev/null
test "$(stat -c '%a' ./output/orders.json)" = 600
# M3 order responses: typed IDs/statuses and optional nested leg objects.
jq -e 'type == "array" and all(.[];
  (.id | type) == "string" and
  (.status | type) == "string" and
  (.legs == null or ((.legs | type) == "array" and
    all(.legs[]; (.id | type) == "string" and (.status | type) == "string"))))' ./output/orders.json > /dev/null

test "$(stat -c '%a' ./output/orders-summary.txt)" = 600
jq -e 'type == "array" and (length <= 10) and all(.[]; .side == "buy" and .asset_class == "crypto")' ./output/orders-filtered.json > /dev/null
test "$(stat -c '%a' ./output/orders-filtered.json)" = 600
jq -e 'type == "array" and all(.[];
  (.legs == null or ((.legs | type) == "array" and
    all(.legs[]; (.id | type) == "string" and (.status | type) == "string"))))' ./output/orders-filtered.json > /dev/null

# A new paper account may have no orders. With an existing order, exercise
# single-order GET using the validated server-assigned UUID from the list.
./target/release/medea order --help > /dev/null
./target/release/medea order get --help > /dev/null
./target/release/medea order cancel --help > /dev/null
# Safety: only offline dry-run, NEVER --confirm in integration scripts.
# Unset credential variables explicitly to prove the preview is offline.
dry_cancel_id=9275f66a-0a1b-4a22-8c55-167416c7cf14
env -u APCA_API_KEY_ID -u APCA_API_SECRET_KEY \
  ./target/release/medea order cancel "$dry_cancel_id" > ./output/order-cancel-dry-run.txt
grep -q '^DRY RUN' ./output/order-cancel-dry-run.txt
grep -q 'no cancellation was sent' ./output/order-cancel-dry-run.txt
grep -q -- '--confirm' ./output/order-cancel-dry-run.txt
test "$(stat -c '%a' ./output/order-cancel-dry-run.txt)" = 600

if jq -e 'length > 0' ./output/orders.json > /dev/null; then
  order_id="$(jq -r '.[0].id' ./output/orders.json)"
  ./target/release/medea order get "$order_id" > ./output/order-summary.txt
  ./target/release/medea order get "$order_id" --nested > ./output/order-nested-summary.txt
  ./target/release/medea order get "$order_id" --nested --out ./output/order.json > ./output/order-written.txt
  grep -q '^Medea — Alpaca paper order$' ./output/order-summary.txt
  grep -q '^Medea — Alpaca paper order$' ./output/order-nested-summary.txt
  jq -e --arg id "$order_id" '(.id | ascii_downcase) == ($id | ascii_downcase) and
    (.status | type) == "string" and
    (.legs == null or ((.legs | type) == "array" and
      all(.legs[]; (.id | type) == "string" and (.status | type) == "string")))' ./output/order.json > /dev/null
  test "$(stat -c '%a' ./output/order.json)" = 600
  test "$(stat -c '%a' ./output/order-summary.txt)" = 600
  test "$(stat -c '%a' ./output/order-nested-summary.txt)" = 600
else
  echo 'Medea: no paper orders available; skipping single-order API check'
fi

./target/release/medea crypto --help > /dev/null
./target/release/medea crypto bars --help > /dev/null
# Two symbols, up to two pages of two bars each: bounded live data check.
crypto_flags=(--symbols BTC/USD,ETH/USD --timeframe 1Hour \
  --start 2026-10-02T00:00:00Z --end 2026-10-02T03:00:00Z \
  --limit 2 --max-pages 2)
./target/release/medea crypto bars "${crypto_flags[@]}" > ./output/crypto-bars-summary.txt
./target/release/medea crypto bars "${crypto_flags[@]}" --out ./output/crypto-bars.json > ./output/crypto-bars-written.txt
./target/release/medea crypto bars "${crypto_flags[@]}" --format jsonl --out ./output/crypto-bars.jsonl > ./output/crypto-bars-jsonl-written.txt
grep -q '^Medea — Alpaca historical crypto bars$' ./output/crypto-bars-summary.txt
jq -e '(.pages | type == "array" and length >= 1 and length <= 2) and (.next_page_token == null or (.next_page_token | type == "string"))' ./output/crypto-bars.json > /dev/null
jq -e 'all(.pages[]; (.bars | type == "object"))' ./output/crypto-bars.json > /dev/null
test "$(stat -c '%a' ./output/crypto-bars.json)" = 600
test "$(stat -c '%a' ./output/crypto-bars.jsonl)" = 600
test -s ./output/crypto-bars.jsonl
jq -e -c '.symbol and .t and (.o | type == "number") and (.v | type == "number")' ./output/crypto-bars.jsonl > /dev/null

# Individual historical trades, two symbols, at most two pages of two trades.
./target/release/medea crypto trades --help > /dev/null
trades_flags=(--symbols BTC/USD,ETH/USD \
  --start 2026-10-02T00:00:00Z --end 2026-10-02T03:00:00Z \
  --limit 2 --max-pages 2)
./target/release/medea crypto trades "${trades_flags[@]}" > ./output/crypto-trades-summary.txt
./target/release/medea crypto trades "${trades_flags[@]}" --out ./output/crypto-trades.json > ./output/crypto-trades-written.txt
./target/release/medea crypto trades "${trades_flags[@]}" --format jsonl --out ./output/crypto-trades.jsonl > ./output/crypto-trades-jsonl-written.txt
grep -q '^Medea — Alpaca historical crypto trades$' ./output/crypto-trades-summary.txt
jq -e '(.pages | type == "array" and length >= 1 and length <= 2) and (.next_page_token == null or (.next_page_token | type == "string"))' ./output/crypto-trades.json > /dev/null
jq -e 'all(.pages[]; (.trades | type == "object"))' ./output/crypto-trades.json > /dev/null
test "$(stat -c '%a' ./output/crypto-trades.json)" = 600
test "$(stat -c '%a' ./output/crypto-trades.jsonl)" = 600
test -s ./output/crypto-trades.jsonl
jq -e -c '.symbol and .i and .t and (.p | type == "number") and (.s | type == "number")' ./output/crypto-trades.jsonl > /dev/null

echo "Medea: Alpaca paper and crypto market-data API integration checks passed"
