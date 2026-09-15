#!/usr/bin/env bash
# Builds the plugin binary for one target and drops it into the bundle's bin directory.
# Usage: packaging/build-binary.sh [target-triple]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUNDLE="$ROOT/plugin/io.github.chimi6.limelight.sdPlugin"
TARGET="${1:-$(rustc -vV | sed -n 's/^host: //p')}"

cd "$ROOT"
rustup target add "$TARGET" > /dev/null 2>&1 || true
echo "==> building limelight-openaction for $TARGET"
cargo build --release --target "$TARGET"

mkdir -p "$BUNDLE/bin"
cp "target/$TARGET/release/limelight-openaction" "$BUNDLE/bin/limelight-openaction-$TARGET"
chmod 755 "$BUNDLE/bin/limelight-openaction-$TARGET"
echo "==> $BUNDLE/bin/limelight-openaction-$TARGET"
