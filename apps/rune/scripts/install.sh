#!/usr/bin/env bash
# Builds Mdow and installs it for the current user.
#
# macOS: a release bundle in ~/Applications (or $DEST).
#   scripts/install.sh
#   DEST=/Applications scripts/install.sh
# Linux: the binary, desktop entry and icon under ~/.local (or $PREFIX).
#   scripts/install.sh
#   PREFIX=/usr/local sudo -E scripts/install.sh
# Either: scripts/install.sh --uninstall
set -euo pipefail

cd "$(dirname "$0")/.."

if [[ "$(uname -s)" == "Darwin" ]]; then
  DEST="${DEST:-$HOME/Applications}"
  APP="Mdow Rune.app"
  if [[ "${1:-}" == "--uninstall" ]]; then
    rm -rf "${DEST:?}/$APP"
    echo "Removed $DEST/$APP"
    exit 0
  fi
  bash scripts/bundle.sh
  mkdir -p "$DEST"
  rm -rf "${DEST:?}/$APP"
  cp -R "dist/$APP" "$DEST/"
  /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$DEST/$APP" || true
  echo "Installed $DEST/$APP"
  exit 0
fi

APP_ID="com.zain.mdow"
PREFIX="${PREFIX:-$HOME/.local}"
BIN="$PREFIX/bin/mdow"
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

echo "Building Mdow (release)…"
cargo build --release --locked

install -Dm755 "${CARGO_TARGET_DIR:-target}/release/mdow" "$BIN"
install -d "$(dirname "$DESKTOP")"
sed "s|@BINDIR@|$PREFIX/bin|g" "data/$APP_ID.desktop.in" >"$DESKTOP"
chmod 644 "$DESKTOP"
install -Dm644 "data/$APP_ID.metainfo.xml" "$METAINFO"
install -Dm644 ../native-linux/assets/icons/mdow-logo.svg "$ICON"
refresh
echo "Installed $BIN"
case ":$PATH:" in
  *":$PREFIX/bin:"*) ;;
  *) echo "Note: $PREFIX/bin is not on your PATH." ;;
esac
