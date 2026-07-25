#!/usr/bin/env bash
# Download / install the sole active MIS kernel (`misc`) for CLRTY-1.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64|amd64) ARCH=x86_64 ;;
  arm64|aarch64) ARCH=arm64 ;;
esac
ASSET="misc-${OS}-${ARCH}"
BIN_DIR="${CLRTY_MIS_BIN:-$ROOT/bin}"
mkdir -p "$BIN_DIR"

if [[ -x "$ROOT/bin/${ASSET}" ]]; then
  SRC="$ROOT/bin/${ASSET}"
elif [[ -x "$ROOT/bin/misc-${OS}-${ARCH}" ]]; then
  SRC="$ROOT/bin/misc-${OS}-${ARCH}"
elif [[ -x "$ROOT/bin/misc-darwin-arm64" && "$OS" == darwin && "$ARCH" == arm64 ]]; then
  SRC="$ROOT/bin/misc-darwin-arm64"
else
  echo "[misc] prebuilt $ASSET not found — building from source…"
  (cd "$ROOT/src/misc" && cargo build --release)
  SRC="$ROOT/src/misc/target/release/misc"
fi

install -m 755 "$SRC" "$BIN_DIR/misc"
# clrtyc is alias of misc only
if [[ -f "${SRC/misc/clrtyc}" ]]; then
  install -m 755 "${SRC/misc/clrtyc}" "$BIN_DIR/clrtyc" 2>/dev/null || cp "$BIN_DIR/misc" "$BIN_DIR/clrtyc"
else
  cp "$BIN_DIR/misc" "$BIN_DIR/clrtyc"
fi
echo "[misc] installed → $BIN_DIR/misc (active_kernel_only=true)"
echo "[misc] verify: $BIN_DIR/misc --help"
"$BIN_DIR/misc" --help | head -5 || true
