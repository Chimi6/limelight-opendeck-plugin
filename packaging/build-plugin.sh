#!/usr/bin/env bash
# Convenience wrapper: build for the host target, then package (and optionally install).
# Usage: packaging/build-plugin.sh [--install]
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
"$HERE/build-binary.sh"
"$HERE/package.sh" "$@"
