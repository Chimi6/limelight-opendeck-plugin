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

Paste this URL into the install field of OpenDeck's Plugins tab, or download \`limelight-opendeck-$VERSION.zip\` and install it from file. Linux x86_64 and aarch64 are included in the same zip.

    https://github.com/Chimi6/limelight-opendeck-plugin/releases/latest/download/limelight-opendeck.zip
NOTES
