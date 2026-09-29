#!/usr/bin/env bash
# Build, sign, notarize and zip "Mdow Native.app" (arm64), then verify the zip it wrote.
#
# Outputs in $DIST_DIR (default: <repo>/dist/native-mac):
#   MdowNative-$VERSION-arm64-mac-beta.zip   website download and Sparkle migration enclosure
#   MdowNative-mac-beta.zip                  stable alias for the website
#   Mdow Native.app.tar.gz (+ .sig)          gpuix updater asset, when CARGO_PACKAGER_SIGN_PRIVATE_KEY is set
#
# Local runs without a Developer ID sign ad-hoc and skip notarization.
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Mac packaging must run on macOS." >&2
  exit 1
fi
if [[ "${ARCH:-$(uname -m)}" != "arm64" ]]; then
  echo "Mac packaging requires an arm64 host (gpuix ships darwin-arm64 only)." >&2
  exit 1
fi

NATIVE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
ROOT_DIR="$(cd "$NATIVE_DIR/../.." && pwd -P)"
APP_NAME="Mdow Native"
EXECUTABLE="MdowNative"
BUNDLE_ID="com.zain.mdow.gpui" # unchanged from the Rust build so settings and Sparkle carry over
MIN_SYSTEM_VERSION="14.0"
VERSION="${VERSION:-$(bun -e "console.log(require('$NATIVE_DIR/package.json').version)")}"
BUILD_NUMBER="${GITHUB_RUN_NUMBER:-0}"
DIST_DIR="${DIST_DIR:-$ROOT_DIR/dist/native-mac}"
CODESIGN="${CODESIGN:-codesign}"
XCRUN="${XCRUN:-xcrun}"

mkdir -p "$DIST_DIR"
DIST_DIR="$(cd "$DIST_DIR" && pwd -P)"
APP="$DIST_DIR/$APP_NAME.app"
CONTENTS="$APP/Contents"
VERSIONED_ZIP="$DIST_DIR/MdowNative-$VERSION-arm64-mac-beta.zip"
ALIAS_ZIP="$DIST_DIR/MdowNative-mac-beta.zip"
UPDATER_TARBALL="$DIST_DIR/$APP_NAME.app.tar.gz"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/mdow-native-mac.XXXXXX")"
trap 'rm -rf -- "$WORK"' EXIT

echo "Compiling $APP_NAME $VERSION"
bun "$NATIVE_DIR/scripts/build.ts" --outfile "$WORK/$EXECUTABLE"
if [[ "$(lipo -archs "$WORK/$EXECUTABLE")" != "arm64" ]]; then
  echo "Expected an arm64-only executable." >&2
  exit 1
fi
BUILT_VERSION="$("$WORK/$EXECUTABLE" --version)"
if [[ "$BUILT_VERSION" != "$VERSION" ]]; then
  echo "apps/native/package.json is $BUILT_VERSION but packaging $VERSION; bump it first." >&2
  exit 1
fi

[[ "$(basename "$APP")" == "$APP_NAME.app" ]] || exit 1
rm -rf -- "$APP" "$VERSIONED_ZIP" "$ALIAS_ZIP" "$UPDATER_TARBALL" "$UPDATER_TARBALL.sig"
mkdir -p "$CONTENTS/MacOS" "$CONTENTS/Resources"
cp "$WORK/$EXECUTABLE" "$CONTENTS/MacOS/$EXECUTABLE"
cp -R "$NATIVE_DIR/assets" "$CONTENTS/Resources/assets"
cp "$ROOT_DIR/apps/desktop/resources/icon.icns" "$CONTENTS/Resources/$EXECUTABLE.icns"

cat >"$CONTENTS/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>$EXECUTABLE</string>
  <key>CFBundleIdentifier</key><string>$BUNDLE_ID</string>
  <key>CFBundleName</key><string>$APP_NAME</string>
  <key>CFBundleDisplayName</key><string>$APP_NAME</string>
  <key>CFBundleIconFile</key><string>$EXECUTABLE.icns</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleVersion</key><string>$BUILD_NUMBER</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.developer-tools</string>
  <key>LSMinimumSystemVersion</key><string>$MIN_SYSTEM_VERSION</string>
  <key>NSPrincipalClass</key><string>NSApplication</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSQuitAlwaysKeepsWindows</key><false/>
  <key>CFBundleDocumentTypes</key>
  <array>
    <dict>
      <key>CFBundleTypeName</key><string>Markdown Document</string>
      <key>CFBundleTypeRole</key><string>Viewer</string>
      <key>LSHandlerRank</key><string>Alternate</string>
      <key>LSItemContentTypes</key><array><string>net.daringfireball.markdown</string></array>
      <key>CFBundleTypeExtensions</key><array><string>md</string><string>markdown</string><string>mdx</string></array>
    </dict>
    <dict>
      <key>CFBundleTypeName</key><string>HTML Document</string>
      <key>CFBundleTypeRole</key><string>Viewer</string>
      <key>LSHandlerRank</key><string>Alternate</string>
      <key>LSItemContentTypes</key><array><string>public.html</string></array>
    </dict>
  </array>
  <key>UTImportedTypeDeclarations</key>
  <array>
    <dict>
      <key>UTTypeIdentifier</key><string>net.daringfireball.markdown</string>
      <key>UTTypeDescription</key><string>Markdown Document</string>
      <key>UTTypeConformsTo</key><array><string>public.plain-text</string></array>
      <key>UTTypeTagSpecification</key>
      <dict>
        <key>public.filename-extension</key><array><string>md</string><string>markdown</string><string>mdx</string></array>
      </dict>
    </dict>
  </array>
</dict>
</plist>
PLIST
plutil -lint "$CONTENTS/Info.plist" >/dev/null

resolve_identity() {
  if [[ -n "${NATIVE_MAC_CODESIGN_IDENTITY:-}" ]]; then
    printf '%s\n' "$NATIVE_MAC_CODESIGN_IDENTITY"
  elif [[ -n "${CSC_NAME:-}" ]]; then
    printf '%s\n' "$CSC_NAME"
  elif [[ -n "${KEYCHAIN_PATH:-}" ]]; then
    security find-identity -v -p codesigning "$KEYCHAIN_PATH" 2>/dev/null |
      sed -n 's/.*"\(Developer ID Application:.*\)".*/\1/p' | head -n 1
  fi
}
IDENTITY="$(resolve_identity)"
if [[ "${CI:-}" == "true" && ( -z "$IDENTITY" || "$IDENTITY" == "-" ) ]]; then
  echo "CI packaging requires a Developer ID Application identity." >&2
  exit 1
fi

if [[ -z "$IDENTITY" ]]; then
  echo "No Developer ID identity; signing ad-hoc."
  "$CODESIGN" --force --sign - "$APP"
else
  # Bun's JIT needs executable memory, and the embedded gpuix addon is extracted and
  # dlopened at runtime, so it cannot carry our team's signature.
  "$CODESIGN" --force --timestamp --options runtime \
    --entitlements "$NATIVE_DIR/scripts/entitlements.plist" \
    --sign "$IDENTITY" "$APP"
fi
"$CODESIGN" --verify --deep --strict --verbose=2 "$APP"

NOTARIZED=false
if [[ -n "$IDENTITY" && -n "${APPLE_ID:-}" && -n "${APPLE_APP_SPECIFIC_PASSWORD:-}" && -n "${APPLE_TEAM_ID:-}" ]]; then
  COPYFILE_DISABLE=1 ditto -c -k --norsrc --noextattr --keepParent "$APP" "$WORK/notary.zip"
  "$XCRUN" notarytool submit "$WORK/notary.zip" \
    --apple-id "$APPLE_ID" \
    --password "$APPLE_APP_SPECIFIC_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" \
    --wait
  "$XCRUN" stapler staple "$APP"
  "$XCRUN" stapler validate "$APP"
  NOTARIZED=true
elif [[ "${CI:-}" == "true" ]]; then
  echo "CI packaging requires Apple notarization credentials." >&2
  exit 1
fi

COPYFILE_DISABLE=1 ditto -c -k --norsrc --noextattr --keepParent "$APP" "$VERSIONED_ZIP"
cp "$VERSIONED_ZIP" "$ALIAS_ZIP"

# The gpuix updater installs from "<App>.app.tar.gz" plus a sibling minisign signature.
COPYFILE_DISABLE=1 tar -czf "$UPDATER_TARBALL" -C "$DIST_DIR" "$APP_NAME.app"
if [[ -n "${CARGO_PACKAGER_SIGN_PRIVATE_KEY:-}" ]]; then
  (cd "$NATIVE_DIR" && bun x packager signer sign "$UPDATER_TARBALL")
  [[ -s "$UPDATER_TARBALL.sig" ]] || {
    echo "Signing did not produce $UPDATER_TARBALL.sig" >&2
    exit 1
  }
elif [[ "${CI:-}" == "true" && -s "$NATIVE_DIR/updater/public-key.txt" ]]; then
  echo "The app embeds an updater key, so CI must sign the update (CARGO_PACKAGER_SIGN_PRIVATE_KEY)." >&2
  exit 1
fi

# Verify what users download, not the working copy.
ditto -x -k "$VERSIONED_ZIP" "$WORK/extract"
EXTRACTED="$WORK/extract/$APP_NAME.app"
"$CODESIGN" --verify --deep --strict --verbose=2 "$EXTRACTED"
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$EXTRACTED/Contents/Info.plist")" == "$VERSION" ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$EXTRACTED/Contents/Info.plist")" == "$BUNDLE_ID" ]]
if [[ "$NOTARIZED" == "true" ]]; then
  spctl -a -vv --type execute "$EXTRACTED"
  "$XCRUN" stapler validate "$EXTRACTED"
fi
EXPECTED_ASSETS="$(cd "$EXTRACTED/Contents/Resources/assets" && pwd -P)"
REPORTED_ASSETS="$("$EXTRACTED/Contents/MacOS/$EXECUTABLE" --verify-assets)"
if [[ "$REPORTED_ASSETS" != "$EXPECTED_ASSETS" ]]; then
  echo "Packaged app found assets at '$REPORTED_ASSETS', expected '$EXPECTED_ASSETS'." >&2
  exit 1
fi

# Font registration, the gpuix addon, Metal and a real window, from the zip.
SMOKE="$(GPUIX_BACKGROUND=1 perl -e 'alarm 45; exec @ARGV' "$EXTRACTED/Contents/MacOS/$EXECUTABLE" --smoke-test 2>&1 || true)"
if [[ "$SMOKE" != *MDOW_SMOKE_OK* ]]; then
  echo "Packaged app smoke test failed:" >&2
  echo "$SMOKE" >&2
  exit 1
fi
echo "Packaged app smoke test passed"

echo "Created:"
ls -1 "$DIST_DIR" | sed 's/^/  /'
