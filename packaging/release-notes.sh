#!/usr/bin/env bash
# Prints the CHANGELOG section for the given version, followed by install instructions.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="$1"

awk -v v="$VERSION" '
	$0 ~ "^## " { printing = ($2 == v) ; next }
	printing { print }
' "$ROOT/CHANGELOG.md"

cat <<NOTES

## Install

Download \`limelight-opendeck-$VERSION.zip\`, open OpenDeck's Plugins tab and install it from file, or paste the asset URL there. Linux x86_64 and aarch64 are included in the same zip.
NOTES
