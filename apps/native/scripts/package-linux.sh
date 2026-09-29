#!/usr/bin/env bash
# Build the Linux x64 AppImage with cargo-packager's npm CLI.
#
# Outputs in $DIST_DIR (default: <repo>/dist/native-linux):
#   mdow-native_$VERSION_x86_64.AppImage (+ .sig)   gpuix updater asset (signed when CARGO_PACKAGER_SIGN_PRIVATE_KEY is set)
#   MdowNative-linux-beta.AppImage                  stable alias for the website
set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "Linux packaging must run on Linux (cargo-packager only builds for its host)." >&2
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

cat >"$WORK/packager.json" <<JSON
{
  "productName": "Mdow Native",
  "version": "$VERSION",
  "identifier": "com.zain.mdow.gpui",
  "description": "A quiet markdown viewer",
  "authors": ["Zain Wania"],
  "category": "Utility",
  "binariesDir": "$WORK/bin",
  "outDir": "$WORK/out",
  "binaries": [{ "path": "$BINARY", "main": true }],
  "icons": ["$ROOT_DIR/apps/desktop/resources/icon.png"],
  "resources": [{ "src": "$NATIVE_DIR/assets", "target": "assets" }],
  "formats": ["appimage"]
}
JSON

(cd "$NATIVE_DIR" && bun x packager --release --config "$WORK/packager.json")
BUILT="$(find "$WORK/out" -maxdepth 2 -name '*.AppImage' | head -n 1)"
if [[ -z "$BUILT" ]]; then
  echo "cargo-packager did not produce an AppImage." >&2
  exit 1
fi
cp "$BUILT" "$DIST_DIR/$APPIMAGE"
chmod +x "$DIST_DIR/$APPIMAGE"
if [[ -f "$BUILT.sig" ]]; then
  cp "$BUILT.sig" "$DIST_DIR/$APPIMAGE.sig"
elif [[ -n "${CARGO_PACKAGER_SIGN_PRIVATE_KEY:-}" ]]; then
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
