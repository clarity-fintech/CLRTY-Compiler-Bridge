#!/usr/bin/env bash
# Install the sole active MIS kernel (`misc`) for CLRTY-1 from this package.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64|amd64) ARCH=x86_64 ;;
  arm64|aarch64) ARCH=arm64 ;;
esac

BIN_DIR="${CLRTY_MIS_BIN:-$ROOT/bin}"
mkdir -p "$BIN_DIR"

resolve_src() {
  local candidates=(
    "$ROOT/bin/misc-${OS}-${ARCH}"
    "$ROOT/bin/misc-darwin-arm64"
    "$ROOT/bin/misc-linux-x86_64"
    "$ROOT/bin/misc-linux-arm64"
  )
  local c
  for c in "${candidates[@]}"; do
    if [[ -x "$c" && ! -L "$c" ]]; then
      echo "$c"
      return 0
    fi
  done
  if [[ -x "$ROOT/bin/misc" && ! -L "$ROOT/bin/misc" ]]; then
    echo "$ROOT/bin/misc"
    return 0
  fi
  return 1
}

link_or_install() {
  local src="$1" dest="$2"
  local src_real dest_real
  src_real="$(cd "$(dirname "$src")" && pwd)/$(basename "$src")"
  if [[ -e "$dest" || -L "$dest" ]]; then
    dest_real="$(cd "$(dirname "$dest")" && pwd)/$(basename "$(readlink "$dest" 2>/dev/null || echo "$dest")")"
    # Same inode / already pointed at platform binary — nothing to do.
    if [[ "$(realpath "$src" 2>/dev/null || echo "$src_real")" == "$(realpath "$dest" 2>/dev/null || echo "")" ]]; then
      return 0
    fi
    rm -f "$dest"
  fi
  # Prefer symlink next to the platform asset when installing into package bin/.
  if [[ "$(dirname "$src_real")" == "$(cd "$BIN_DIR" && pwd)" ]]; then
    ln -sfn "$(basename "$src")" "$dest"
  else
    install -m 755 "$src" "$dest"
  fi
}

SRC=""
if SRC="$(resolve_src)"; then
  :
else
  echo "[misc] prebuilt for ${OS}-${ARCH} not found — building from source…"
  if [[ ! -f "$ROOT/src/misc/Cargo.toml" ]]; then
    echo "[misc] missing src/misc — clone https://github.com/clarity-fintech/CLRTY-MIS-Kernel" >&2
    exit 1
  fi
  (cd "$ROOT/src/misc" && cargo build --release)
  SRC="$ROOT/src/misc/target/release/misc"
fi

link_or_install "$SRC" "$BIN_DIR/misc"

CLRTYC_SRC="${SRC/misc/clrtyc}"
if [[ -x "$CLRTYC_SRC" && "$CLRTYC_SRC" != "$SRC" ]]; then
  link_or_install "$CLRTYC_SRC" "$BIN_DIR/clrtyc"
else
  # Alias: clrtyc → same binary as misc
  if [[ -L "$BIN_DIR/misc" ]]; then
    ln -sfn "$(readlink "$BIN_DIR/misc")" "$BIN_DIR/clrtyc"
  else
    cp -f "$BIN_DIR/misc" "$BIN_DIR/clrtyc"
    chmod 755 "$BIN_DIR/clrtyc"
  fi
fi

echo "[misc] installed → $BIN_DIR/misc (active_kernel_only=true)"
echo "[misc] smoke: $BIN_DIR/misc src/misc.mis --check --compact-letters"
"$BIN_DIR/misc" --help | head -5 || true
if [[ -f "$ROOT/src/misc.mis" ]]; then
  "$BIN_DIR/misc" "$ROOT/src/misc.mis" --check --compact-letters
fi
