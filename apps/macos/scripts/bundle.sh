#!/usr/bin/env bash
# Builds "Mdow for Mac.app" into dist/. Signs ad hoc unless SIGN_IDENTITY is set.
#   CONFIGURATION=debug scripts/bundle.sh    # faster, unoptimised build
#   SIGN_IDENTITY="Developer ID Application: …" scripts/bundle.sh
# The version comes from ./VERSION (VERSION=… overrides it).
set -euo pipefail

cd "$(dirname "$0")/.."
CONFIGURATION="${CONFIGURATION:-release}"
VERSION="${VERSION:-$(tr -d '[:space:]' < VERSION)}"
BUILD="${BUILD:-$(git rev-list --count HEAD 2>/dev/null || echo 1)}"
APP="dist/Mdow for Mac.app"
SPARKLE=".build/artifacts/sparkle/Sparkle/Sparkle.xcframework/macos-arm64_x86_64/Sparkle.framework"

bash scripts/fetch-pele.sh
swift build -c "$CONFIGURATION" --product Mdow
BIN="$(swift build -c "$CONFIGURATION" --show-bin-path)/Mdow"
if [[ ! -d "$SPARKLE" ]]; then
  echo "Missing $SPARKLE (SwiftPM downloads it when resolving packages)" >&2
  exit 1
fi

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$APP/Contents/Frameworks"
# SwiftPM records the deployment target as the SDK version, which makes macOS run the app in
# its compatibility look (no Liquid Glass). Record the SDK it was actually built with.
SDK_VERSION="$(xcrun --sdk macosx --show-sdk-version)"
xcrun vtool -set-build-version macos 15.0 "$SDK_VERSION" -replace -output "$APP/Contents/MacOS/Mdow" "$BIN"
sed -e "s/__VERSION__/$VERSION/" -e "s/__BUILD__/$BUILD/" Resources/Info.plist > "$APP/Contents/Info.plist"
cp Resources/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"
cp Resources/vendor/pele.global.js "$APP/Contents/Resources/pele.global.js"
# Inter for the interface and Geist Mono for code, registered at launch (Theme.registerFonts).
ditto Resources/Fonts "$APP/Contents/Resources/Fonts"
# SwiftMath finds its fonts through Bundle.module, which looks in Contents/Resources. Only the
# Latin Modern font is used, so the other ten are left out.
MATH_BUNDLE="$APP/Contents/Resources/SwiftMath_SwiftMath.bundle"
ditto "$(dirname "$BIN")/SwiftMath_SwiftMath.bundle" "$MATH_BUNDLE"
rm -rf "$MATH_BUNDLE/Contents/_CodeSignature"
# Xcode versions lay the bundle out differently (flat or with Contents/Resources).
MATH_FONTS="$(find "$MATH_BUNDLE" -type d -name mathFonts.bundle | head -1)"
if [[ -z "$MATH_FONTS" ]]; then
  echo "Missing mathFonts.bundle in $MATH_BUNDLE:" >&2
  find "$MATH_BUNDLE" -maxdepth 3 >&2
  exit 1
fi
find "$MATH_FONTS" \( -name '*.otf' -o -name '*.plist' \) ! -name 'latinmodern-math.*' -delete

# Sparkle. The executable finds it through its @executable_path/../Frameworks rpath (set in
# Package.swift). The app isn't sandboxed, so it doesn't need Sparkle's XPC services, and
# headers aren't needed at run time.
FRAMEWORK="$APP/Contents/Frameworks/Sparkle.framework"
ditto "$SPARKLE" "$FRAMEWORK"
for part in XPCServices Headers PrivateHeaders Modules; do
  rm -rf "${FRAMEWORK:?}/$part" "${FRAMEWORK:?}/Versions/B/$part"
done

# Sign inside out, everything with the same identity, as Sparkle documents. A Developer ID
# build gets the hardened runtime and a secure timestamp, which notarization requires. An ad hoc
# build gets neither: ad hoc signatures have no Team ID, so the hardened runtime's library
# validation would refuse to load the embedded Sparkle.framework.
IDENTITY="${SIGN_IDENTITY:--}"
if [[ "$IDENTITY" == "-" ]]; then
  SIGN_FLAGS=(--timestamp=none)
else
  SIGN_FLAGS=(--options runtime --timestamp)
fi
sign() { codesign --force "${SIGN_FLAGS[@]}" --sign "$IDENTITY" "$@"; }
sign "$FRAMEWORK/Versions/B/Autoupdate"
sign "$FRAMEWORK/Versions/B/Updater.app"
sign "$FRAMEWORK"
sign "$APP"
echo "Built $APP ($VERSION, build $BUILD)"
