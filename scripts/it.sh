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

test -s ./output/account-summary.txt
grep -q '^Medea — Alpaca paper account$' ./output/account-summary.txt
test -s ./output/account.json
jq -e '.id and .status and .cash and .equity and .buying_power' ./output/account.json > /dev/null
test "$(stat -c '%a' ./output/account.json)" = 600
test "$(stat -c '%a' ./output/account-summary.txt)" = 600

test -s ./output/positions-summary.txt
grep -q '^Medea — Alpaca paper positions$' ./output/positions-summary.txt
jq -e 'type == "array"' ./output/positions.json > /dev/null
test "$(stat -c '%a' ./output/positions.json)" = 600
test "$(stat -c '%a' ./output/positions-summary.txt)" = 600

test -s ./output/assets-summary.txt
grep -q '^Medea — Alpaca paper assets$' ./output/assets-summary.txt
jq -e 'type == "array" and all(.[]; .class == "crypto" and (.tradable | type == "boolean"))' ./output/assets.json > /dev/null
test "$(stat -c '%a' ./output/assets.json)" = 600
test "$(stat -c '%a' ./output/assets-summary.txt)" = 600

echo "Medea: Alpaca paper API integration checks passed"
