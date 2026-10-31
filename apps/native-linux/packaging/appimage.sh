#!/usr/bin/env bash
# Build the Mdow for Linux AppImage. Run on Ubuntu 24.04 (CI, or `packaging/docker-appimage.sh`):
# the AppImage bundles GTK and libadwaita from the build system and runs on any distro with the
# same or newer glibc (2.39: Ubuntu 24.04+, Debian 13+, Fedora 40+, Arch).
#
# Needs: cargo, pkg-config, libgtk-4-dev, libadwaita-1-dev, adwaita-icon-theme, curl, file.
# Output: $DIST_DIR/Mdow-<version>-x86_64.AppImage (default <repo>/dist/native-linux).
set -euo pipefail

APP_ID="com.zain.mdow.Linux"
APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
ROOT_DIR="$(cd "$APP_DIR/../.." && pwd -P)"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$APP_DIR/Cargo.toml" | head -1)"
DIST_DIR="${DIST_DIR:-$ROOT_DIR/dist/native-linux}"
WORK="${WORK:-$(mktemp -d "${TMPDIR:-/tmp}/mdow-appimage.XXXXXX")}"
APPDIR="$WORK/AppDir"
TOOLS="$WORK/tools"
export APPIMAGE_EXTRACT_AND_RUN=1 # linuxdeploy is itself an AppImage; containers have no FUSE

echo "Building Mdow for Linux $VERSION"
cargo build --release --locked --manifest-path "$APP_DIR/Cargo.toml"

mkdir -p "$TOOLS" "$DIST_DIR"
fetch() {
  [[ -x "$TOOLS/$1" ]] || { curl -fsSL -o "$TOOLS/$1" "$2"; chmod +x "$TOOLS/$1"; }
}
fetch linuxdeploy-x86_64.AppImage \
  https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage
fetch linuxdeploy-plugin-gtk.sh \
  https://raw.githubusercontent.com/linuxdeploy/linuxdeploy-plugin-gtk/master/linuxdeploy-plugin-gtk.sh
export PATH="$TOOLS:$PATH"

rm -rf "$APPDIR"
install -Dm755 "${CARGO_TARGET_DIR:-$APP_DIR/target}/release/mdow-linux" "$APPDIR/usr/bin/mdow-linux"
install -d "$APPDIR/usr/share/applications"
sed "s|@BINDIR@/||" "$APP_DIR/data/$APP_ID.desktop.in" >"$APPDIR/usr/share/applications/$APP_ID.desktop"
install -Dm644 "$APP_DIR/data/$APP_ID.metainfo.xml" "$APPDIR/usr/share/metainfo/$APP_ID.appdata.xml"
install -Dm644 "$APP_DIR/assets/icons/mdow-logo.svg" \
  "$APPDIR/usr/share/icons/hicolor/scalable/apps/$APP_ID.svg"

# The symbolic icons the UI uses, from Adwaita, so hosts without that theme still show them.
ICONS="$(grep -rhoE '"[a-z-]+-symbolic"' "$APP_DIR/src" | tr -d '"' | sort -u)"
for icon in $ICONS; do
  src="$(find /usr/share/icons/Adwaita -name "$icon.svg" -print -quit 2>/dev/null || true)"
  [[ -n "$src" ]] && install -Dm644 "$src" "$APPDIR/usr/share/icons/hicolor/scalable/actions/$icon.svg"
done

DEPLOY_GTK_VERSION=4 linuxdeploy-x86_64.AppImage --appdir "$APPDIR" --plugin gtk \
  --desktop-file "$APPDIR/usr/share/applications/$APP_ID.desktop" \
  --icon-file "$APPDIR/usr/share/icons/hicolor/scalable/apps/$APP_ID.svg"

# Mdow plays no media; GTK's GStreamer module would load the host's GStreamer against the
# bundled GLib and fail.
rm -rf "$APPDIR"/usr/lib/gtk-4.0/*/media "$APPDIR"/usr/lib/gtk-4.0/*/printbackends \
  "$APPDIR"/usr/lib/libgst* "$APPDIR"/usr/lib/libcups*

# The GTK plugin forces X11 ("Wayland is unstable") and pins GTK_THEME, which breaks
# libadwaita's own light/dark handling. Drop both.
sed -i '/GDK_BACKEND\|GTK_THEME/d' "$APPDIR"/apprun-hooks/*gtk*.sh

OUTPUT="$DIST_DIR/Mdow-$VERSION-x86_64.AppImage"
rm -f "$OUTPUT"
(cd "$WORK" && LDAI_OUTPUT="$OUTPUT" ARCH=x86_64 linuxdeploy-x86_64.AppImage --appdir "$APPDIR" --output appimage)
chmod +x "$OUTPUT"
echo "Built $OUTPUT"
