#!/usr/bin/env bash
# Build the Linux x64 AppImage: an AppDir around the compiled binary, packed by appimagetool.
#
# linuxdeploy is not used. The binary bundles every library it needs, so there is nothing for it
# to collect, and the patchelf it ships (0.8, from 2015) rewrites Bun 1.4.3+ executables into
# ones that segfault on start.
#
# Outputs in $DIST_DIR (default: <repo>/dist/native-linux):
#   mdow-native_$VERSION_x86_64.AppImage (+ .sig)   gpuix updater asset (signed when CARGO_PACKAGER_SIGN_PRIVATE_KEY is set)
#   MdowNative-linux-beta.AppImage                  stable alias for the website
set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "Linux packaging must run on Linux (bun compiles and appimagetool runs for the host)." >&2
  exit 1
fi
case "${ARCH:-$(uname -m)}" in
  x86_64 | amd64) ;;
  *)
    echo "Linux packaging requires x86_64 (gpuix ships linux-x64-gnu only)." >&2
    exit 1
    ;;
esac

NATIVE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
ROOT_DIR="$(cd "$NATIVE_DIR/../.." && pwd -P)"
VERSION="${VERSION:-$(bun -e "console.log(require('$NATIVE_DIR/package.json').version)")}"
DIST_DIR="${DIST_DIR:-$ROOT_DIR/dist/native-linux}"
BINARY="mdow-native"
APPIMAGE="${BINARY}_${VERSION}_x86_64.AppImage"
ALIAS="MdowNative-linux-beta.AppImage"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/mdow-native-linux.XXXXXX")"
trap 'rm -rf -- "$WORK"' EXIT

mkdir -p "$DIST_DIR"
DIST_DIR="$(cd "$DIST_DIR" && pwd -P)"
rm -f -- "$DIST_DIR/$APPIMAGE" "$DIST_DIR/$APPIMAGE.sig" "$DIST_DIR/$ALIAS"

echo "Compiling Mdow Native $VERSION"
mkdir -p "$WORK/bin"
bun "$NATIVE_DIR/scripts/build.ts" --outfile "$WORK/bin/$BINARY"
BUILT_VERSION="$("$WORK/bin/$BINARY" --version)"
if [[ "$BUILT_VERSION" != "$VERSION" ]]; then
  echo "apps/native/package.json is $BUILT_VERSION but packaging $VERSION; bump it first." >&2
  exit 1
fi

# appimagetool and the AppImage runtime, pinned and checksummed, cached between runs.
APPIMAGETOOL_URL="https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage"
APPIMAGETOOL_SHA256="ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0"
RUNTIME_URL="https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-x86_64"
RUNTIME_SHA256="2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d"
TOOLS="${XDG_CACHE_HOME:-$HOME/.cache}/mdow-native-appimage"
mkdir -p "$TOOLS"
fetch() {
  local url="$1" sha="$2" out="$3"
  if echo "$sha  $out" | sha256sum --check --status 2>/dev/null; then return; fi
  curl -fsSL "$url" -o "$out.part"
  if ! echo "$sha  $out.part" | sha256sum --check --status; then
    echo "Checksum mismatch for $url" >&2
    exit 1
  fi
  mv "$out.part" "$out"
}
fetch "$APPIMAGETOOL_URL" "$APPIMAGETOOL_SHA256" "$TOOLS/appimagetool"
fetch "$RUNTIME_URL" "$RUNTIME_SHA256" "$TOOLS/runtime-x86_64"
chmod +x "$TOOLS/appimagetool"

# The same AppDir layout cargo-packager produced. No resources: the fonts ship inside the binary
# and are extracted on first launch.
APPDIR="$WORK/$BINARY.AppDir"
ICON_DIR="usr/share/icons/hicolor/1024x1024/apps"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" "$APPDIR/$ICON_DIR"
cp "$WORK/bin/$BINARY" "$APPDIR/usr/bin/$BINARY"
cp "$ROOT_DIR/apps/desktop/resources/icon.png" "$APPDIR/$ICON_DIR/$BINARY.png"
cat >"$APPDIR/usr/share/applications/$BINARY.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Mdow Native
Comment=A quiet markdown viewer
Exec=$BINARY
Icon=$BINARY
Categories=Utility;
Terminal=false
DESKTOP
ln -s "usr/bin/$BINARY" "$APPDIR/AppRun"
ln -s "usr/share/applications/$BINARY.desktop" "$APPDIR/$BINARY.desktop"
ln -s "$ICON_DIR/$BINARY.png" "$APPDIR/$BINARY.png"
cp "$APPDIR/$ICON_DIR/$BINARY.png" "$APPDIR/.DirIcon"

mkdir -p "$WORK/out"
ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$TOOLS/appimagetool" --no-appstream \
  --runtime-file "$TOOLS/runtime-x86_64" "$APPDIR" "$WORK/out/$APPIMAGE"
cp "$WORK/out/$APPIMAGE" "$DIST_DIR/$APPIMAGE"
chmod +x "$DIST_DIR/$APPIMAGE"
if [[ -n "${CARGO_PACKAGER_SIGN_PRIVATE_KEY:-}" ]]; then
  (cd "$NATIVE_DIR" && bun x packager signer sign "$DIST_DIR/$APPIMAGE")
fi
if [[ "${CI:-}" == "true" && -s "$NATIVE_DIR/updater/public-key.txt" && ! -s "$DIST_DIR/$APPIMAGE.sig" ]]; then
  echo "The app embeds an updater key, so CI must sign the update (CARGO_PACKAGER_SIGN_PRIVATE_KEY)." >&2
  exit 1
fi
cp "$DIST_DIR/$APPIMAGE" "$DIST_DIR/$ALIAS"

# Launch the AppImage itself: the gpuix addon, Vulkan/X11 and a real window.
SMOKE="$(GPUIX_BACKGROUND=1 APPIMAGE_EXTRACT_AND_RUN=1 timeout 60 "$DIST_DIR/$APPIMAGE" --smoke-test 2>&1 || true)"
if [[ "$SMOKE" != *MDOW_SMOKE_OK* ]]; then
  echo "AppImage smoke test failed:" >&2
  echo "$SMOKE" >&2
  exit 1
fi
echo "AppImage smoke test passed"

echo "Created:"
ls -1 "$DIST_DIR" | sed 's/^/  /'
