#!/usr/bin/env bash
# Verifies Cargo.toml, manifest.json and (optionally) a given version string all agree.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

CRATE="$(grep -m1 '^version' "$ROOT/Cargo.toml" | cut -d'"' -f2)"
MANIFEST="$(jq -r .Version "$ROOT"/plugin/*/manifest.json)"
EXPECTED="${1:-$CRATE}"

if [[ "$CRATE" != "$MANIFEST" ]]; then
	echo "Cargo.toml version ($CRATE) does not match manifest.json Version ($MANIFEST)" >&2
	exit 1
fi
if [[ "$CRATE" != "$EXPECTED" ]]; then
	echo "crate version ($CRATE) does not match expected version ($EXPECTED)" >&2
	exit 1
fi
echo "version $CRATE"
