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

test -s ./output/account-summary.txt
grep -q '^Medea — Alpaca paper account$' ./output/account-summary.txt
test -s ./output/account.json
jq -e '.id and .status and .cash and .equity and .buying_power' ./output/account.json > /dev/null
test "$(stat -c '%a' ./output/account.json)" = 600
test "$(stat -c '%a' ./output/account-summary.txt)" = 600

echo "Medea: Alpaca paper API integration checks passed"
