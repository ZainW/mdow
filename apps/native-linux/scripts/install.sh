#!/usr/bin/env bash
# Build Mdow for Linux and install it for the current user (or under $PREFIX).
#
#   scripts/install.sh               # ~/.local
#   PREFIX=/usr/local sudo -E scripts/install.sh
#   scripts/install.sh --uninstall
set -euo pipefail

APP_ID="com.zain.mdow.Linux"
APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
ROOT_DIR="$(cd "$APP_DIR/../.." && pwd -P)"
PREFIX="${PREFIX:-$HOME/.local}"
BIN="$PREFIX/bin/mdow-linux"
DESKTOP="$PREFIX/share/applications/$APP_ID.desktop"
METAINFO="$PREFIX/share/metainfo/$APP_ID.metainfo.xml"
ICON="$PREFIX/share/icons/hicolor/scalable/apps/$APP_ID.svg"

refresh() {
  command -v update-desktop-database >/dev/null && update-desktop-database -q "$PREFIX/share/applications" || true
  # Only refresh an icon cache that already exists; a new one in ~/.local would go stale.
  if [[ -f "$PREFIX/share/icons/hicolor/icon-theme.cache" ]] && command -v gtk-update-icon-cache >/dev/null; then
    gtk-update-icon-cache -q -t -f "$PREFIX/share/icons/hicolor" || true
  fi
}

if [[ "${1:-}" == "--uninstall" ]]; then
  rm -f -- "$BIN" "$DESKTOP" "$METAINFO" "$ICON"
  refresh
  echo "Removed Mdow from $PREFIX"
  exit 0
fi

echo "Building Mdow for Linux (release)…"
cargo build --release --locked --manifest-path "$APP_DIR/Cargo.toml"

install -Dm755 "$APP_DIR/target/release/mdow-linux" "$BIN"
install -d "$(dirname "$DESKTOP")"
sed "s|@BINDIR@|$PREFIX/bin|g" "$APP_DIR/data/$APP_ID.desktop.in" >"$DESKTOP"
chmod 644 "$DESKTOP"
install -Dm644 "$APP_DIR/data/$APP_ID.metainfo.xml" "$METAINFO"
install -Dm644 "$ROOT_DIR/apps/native/assets/icons/mdow-logo.svg" "$ICON"
refresh

echo "Installed Mdow to $BIN"
echo "Make it the default for markdown with:"
echo "  xdg-mime default $APP_ID.desktop text/markdown"
