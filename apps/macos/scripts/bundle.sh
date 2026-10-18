#!/usr/bin/env bash
# Builds "Mdow for Mac.app" into dist/. Signs ad hoc unless SIGN_IDENTITY is set.
#   CONFIGURATION=debug scripts/bundle.sh    # faster, unoptimised build
set -euo pipefail

cd "$(dirname "$0")/.."
CONFIGURATION="${CONFIGURATION:-release}"
VERSION="${VERSION:-0.1.0}"
BUILD="${BUILD:-$(git rev-list --count HEAD 2>/dev/null || echo 1)}"
APP="dist/Mdow for Mac.app"

bash scripts/fetch-pele.sh
swift build -c "$CONFIGURATION" --product Mdow
BIN="$(swift build -c "$CONFIGURATION" --show-bin-path)/Mdow"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
# SwiftPM records the deployment target as the SDK version, which makes macOS run the app in
# its compatibility look (no Liquid Glass). Record the SDK it was actually built with.
SDK_VERSION="$(xcrun --sdk macosx --show-sdk-version)"
xcrun vtool -set-build-version macos 15.0 "$SDK_VERSION" -replace -output "$APP/Contents/MacOS/Mdow" "$BIN"
sed -e "s/__VERSION__/$VERSION/" -e "s/__BUILD__/$BUILD/" Resources/Info.plist > "$APP/Contents/Info.plist"
cp Resources/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"
cp Resources/vendor/pele.global.js "$APP/Contents/Resources/pele.global.js"
# SwiftMath finds its fonts through Bundle.module, which looks in Contents/Resources. Only the
# Latin Modern font is used, so the other ten are left out.
MATH_BUNDLE="$APP/Contents/Resources/SwiftMath_SwiftMath.bundle"
ditto "$(dirname "$BIN")/SwiftMath_SwiftMath.bundle" "$MATH_BUNDLE"
rm -rf "$MATH_BUNDLE/Contents/_CodeSignature"
find "$MATH_BUNDLE/Contents/Resources/mathFonts.bundle" \( -name '*.otf' -o -name '*.plist' \) \
  ! -name 'latinmodern-math.*' -delete

codesign --force --options runtime --timestamp=none --sign "${SIGN_IDENTITY:--}" "$APP"
echo "Built $APP"
