#!/usr/bin/env bash
# Build the Mdow AppImage for Linux. Run on Ubuntu 24.04 (CI does): the AppImage then runs on
# any distro with the same or newer glibc (2.39: Ubuntu 24.04+, Debian 13+, Fedora 40+, Arch).
#
# The binary carries its own renderer, fonts and text stack, and finds the desktop's Wayland,
# X11 and Vulkan or GL libraries at run time, so there is nothing else to bundle.
#
# Needs: cargo, a C compiler, curl, file.
# Output: $DIST_DIR/Mdow-<version>-x86_64.AppImage (default <repo>/dist/linux).
set -euo pipefail

APP_ID="com.zain.mdow"
APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
ROOT_DIR="$(cd "$APP_DIR/../.." && pwd -P)"
# The release version lives in the root package.json, as it does for the Mac bundle.
VERSION="${VERSION:-$(sed -n 's/.*"version": *"\([^"]*\)".*/\1/p' "$ROOT_DIR/package.json" | head -1)}"
DIST_DIR="${DIST_DIR:-$ROOT_DIR/dist/linux}"
WORK="${WORK:-$(mktemp -d "${TMPDIR:-/tmp}/mdow-appimage.XXXXXX")}"
APPDIR="$WORK/AppDir"
TOOLS="$WORK/tools"
export APPIMAGE_EXTRACT_AND_RUN=1 # linuxdeploy is itself an AppImage; containers have no FUSE

echo "Building Mdow $VERSION"
(cd "$APP_DIR" && cargo build --release --locked)

mkdir -p "$TOOLS" "$DIST_DIR"
[[ -x "$TOOLS/linuxdeploy-x86_64.AppImage" ]] || {
  curl -fsSL -o "$TOOLS/linuxdeploy-x86_64.AppImage" \
    https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage
  chmod +x "$TOOLS/linuxdeploy-x86_64.AppImage"
}

rm -rf "$APPDIR"
install -Dm755 "${CARGO_TARGET_DIR:-$APP_DIR/target}/release/mdow" "$APPDIR/usr/bin/mdow"
install -d "$APPDIR/usr/share/applications"
sed "s|@BINDIR@/||" "$APP_DIR/data/$APP_ID.desktop.in" >"$APPDIR/usr/share/applications/$APP_ID.desktop"
install -Dm644 "$APP_DIR/data/$APP_ID.metainfo.xml" "$APPDIR/usr/share/metainfo/$APP_ID.appdata.xml"
install -Dm644 "$APP_DIR/assets/icons/mdow-logo.svg" \
  "$APPDIR/usr/share/icons/hicolor/scalable/apps/$APP_ID.svg"

OUTPUT="$DIST_DIR/Mdow-$VERSION-x86_64.AppImage"
rm -f "$OUTPUT"
(cd "$WORK" && LDAI_OUTPUT="$OUTPUT" ARCH=x86_64 "$TOOLS/linuxdeploy-x86_64.AppImage" \
  --appdir "$APPDIR" \
  --desktop-file "$APPDIR/usr/share/applications/$APP_ID.desktop" \
  --icon-file "$APPDIR/usr/share/icons/hicolor/scalable/apps/$APP_ID.svg" \
  --output appimage)
chmod +x "$OUTPUT"
echo "Built $OUTPUT"
