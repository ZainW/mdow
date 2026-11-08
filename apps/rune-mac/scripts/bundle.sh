#!/usr/bin/env bash
# Builds `dist/Mdow Rune.app`: the release binary, Info.plist (document types, version) and the
# app icon. Signed ad hoc unless SIGN_IDENTITY is set.
#   scripts/bundle.sh
#   CONFIGURATION=debug scripts/bundle.sh
set -euo pipefail

cd "$(dirname "$0")/.."
CONFIGURATION="${CONFIGURATION:-release}"
VERSION="${VERSION:-$(sed -n 's/.*"version": *"\([^"]*\)".*/\1/p' ../../package.json | head -1)}"
BUILD="$(git rev-list --count HEAD 2>/dev/null || echo 1)"
APP="dist/Mdow Rune.app"

if [[ "$CONFIGURATION" == "release" ]]; then
  cargo build --release
else
  cargo build
fi

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "target/$CONFIGURATION/mdow" "$APP/Contents/MacOS/mdow"
sed -e "s/__VERSION__/$VERSION/" -e "s/__BUILD__/$BUILD/" Resources/Info.plist > "$APP/Contents/Info.plist"
cp ../macos/Resources/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"

if [[ -n "${SIGN_IDENTITY:-}" ]]; then
  codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" "$APP"
else
  codesign --force --sign - "$APP"
fi
echo "Built $APP ($VERSION, build $BUILD)"
