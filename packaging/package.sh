#!/usr/bin/env bash
# Assembles the .sdPlugin zip from whatever plugin binaries are present in the bundle's bin directory.
# Fetches a matching keylightd for each target from the LimeLight release, regenerates icons, zips.
# Usage: packaging/package.sh [--install]
#   --install  also copy the bundle into the local OpenDeck plugins directory
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
UUID="io.github.chimi6.limelight"
BUNDLE="$ROOT/plugin/$UUID.sdPlugin"
KEYLIGHTD_VERSION="${KEYLIGHTD_VERSION:-0.2.0}"
KEYLIGHTD_REPO="Chimi6/limelight-linux-elgato-lights-controller"
VERSION="$(grep -m1 '^version' "$ROOT/Cargo.toml" | cut -d'"' -f2)"

cd "$ROOT"
shopt -s nullglob
BINARIES=("$BUNDLE"/bin/limelight-openaction-*)
if [[ ${#BINARIES[@]} -eq 0 ]]; then
	echo "no plugin binaries in $BUNDLE/bin; run packaging/build-binary.sh first" >&2
	exit 1
fi

fetch_daemon() {
	local target="$1" arch
	case "$target" in
		x86_64-unknown-linux-gnu) arch="x86_64" ;;
		aarch64-unknown-linux-gnu) arch="aarch64" ;;
		*) return 0 ;;
	esac
	local dest="$BUNDLE/bin/keylightd-$target"
	[[ -f "$dest" ]] && return 0
	local tarball="LimeLight-$KEYLIGHTD_VERSION-$arch.tar.gz"
	local url="https://github.com/$KEYLIGHTD_REPO/releases/download/v$KEYLIGHTD_VERSION/$tarball"
	local tmp
	tmp="$(mktemp -d)"
	echo "==> fetching keylightd $KEYLIGHTD_VERSION for $arch"
	if curl -fsSL "$url" -o "$tmp/$tarball"; then
		tar -xzf "$tmp/$tarball" -C "$tmp"
		cp "$tmp/LimeLight-$KEYLIGHTD_VERSION-$arch/keylightd" "$dest"
		chmod 755 "$dest"
	else
		echo "warning: no keylightd release for $arch; that build will use keylightd from PATH" >&2
	fi
	rm -rf "$tmp"
}

for binary in "${BINARIES[@]}"; do
	fetch_daemon "${binary##*/limelight-openaction-}"
done
chmod 755 "$BUNDLE"/bin/*

echo "==> regenerating icons"
cargo run -q --release --bin gen-icons -- "$BUNDLE/images" > /dev/null

mkdir -p "$ROOT/dist"
ZIP="$ROOT/dist/limelight-opendeck-$VERSION.zip"
rm -f "$ZIP"
(cd "$ROOT/plugin" && zip -qr "$ZIP" "$UUID.sdPlugin")
echo "==> wrote $ZIP"

if [[ "${1:-}" == "--install" ]]; then
	if [[ -d "$HOME/.var/app/me.amankhanna.opendeck/config/opendeck" ]]; then
		PLUGINS="$HOME/.var/app/me.amankhanna.opendeck/config/opendeck/plugins"
	else
		PLUGINS="${XDG_CONFIG_HOME:-$HOME/.config}/opendeck/plugins"
	fi
	mkdir -p "$PLUGINS"
	rm -rf "$PLUGINS/$UUID.sdPlugin"
	cp -r "$BUNDLE" "$PLUGINS/"
	echo "==> installed to $PLUGINS/$UUID.sdPlugin (restart OpenDeck)"
fi
