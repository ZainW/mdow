#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Native Mac beta packaging must run on macOS." >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_DIR="$ROOT_DIR/apps/native-mac"
DIST_DIR="$ROOT_DIR/dist/native-mac"
APP_NAME="MdowNative"
BUNDLE_ID="com.zain.mdow.native"
MIN_SYSTEM_VERSION="14.0"
VERSION="${VERSION:-}"
BUILD_NUMBER="${GITHUB_RUN_NUMBER:-0}"
NODE_BINARY="${NODE_BINARY:-node}"

if [[ -z "$VERSION" ]]; then
  VERSION="$("$NODE_BINARY" -e "const fs=require('fs'); const pkg=JSON.parse(fs.readFileSync('$ROOT_DIR/apps/desktop/package.json','utf8')); process.stdout.write(pkg.version)")"
fi

ARCH="$(uname -m)"
case "$ARCH" in
  arm64) RELEASE_ARCH="arm64" ;;
  x86_64) RELEASE_ARCH="x64" ;;
  *) RELEASE_ARCH="$ARCH" ;;
esac

APP_BUNDLE="$DIST_DIR/$APP_NAME.app"
APP_CONTENTS="$APP_BUNDLE/Contents"
APP_MACOS="$APP_CONTENTS/MacOS"
APP_BINARY="$APP_MACOS/$APP_NAME"
INFO_PLIST="$APP_CONTENTS/Info.plist"
VERSIONED_ZIP="$DIST_DIR/$APP_NAME-$VERSION-$RELEASE_ARCH-mac-beta.zip"
ALIAS_ZIP="$DIST_DIR/$APP_NAME-mac-beta.zip"

echo "Running native core checks"
swift run --package-path "$APP_DIR" MdowNativeCoreChecks

echo "Building $APP_NAME release binary"
swift build --package-path "$APP_DIR" -c release
BUILD_BINARY="$(swift build --package-path "$APP_DIR" -c release --show-bin-path)/$APP_NAME"

rm -rf "$APP_BUNDLE"
mkdir -p "$APP_MACOS"
cp "$BUILD_BINARY" "$APP_BINARY"
chmod +x "$APP_BINARY"

cat >"$INFO_PLIST" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key>
  <string>$APP_NAME</string>
  <key>CFBundleIdentifier</key>
  <string>$BUNDLE_ID</string>
  <key>CFBundleName</key>
  <string>$APP_NAME</string>
  <key>CFBundleShortVersionString</key>
  <string>$VERSION</string>
  <key>CFBundleVersion</key>
  <string>$BUILD_NUMBER</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>LSMinimumSystemVersion</key>
  <string>$MIN_SYSTEM_VERSION</string>
  <key>NSPrincipalClass</key>
  <string>NSApplication</string>
  <key>NSQuitAlwaysKeepsWindows</key>
  <false/>
  <key>CFBundleDocumentTypes</key>
  <array>
    <dict>
      <key>CFBundleTypeName</key>
      <string>Markdown Document</string>
      <key>CFBundleTypeExtensions</key>
      <array>
        <string>md</string>
        <string>markdown</string>
        <string>mdx</string>
      </array>
      <key>CFBundleTypeRole</key>
      <string>Viewer</string>
    </dict>
  </array>
</dict>
</plist>
PLIST

resolve_signing_identity() {
  if [[ -n "${NATIVE_MAC_CODESIGN_IDENTITY:-}" ]]; then
    printf '%s' "$NATIVE_MAC_CODESIGN_IDENTITY"
    return
  fi

  if [[ -n "${CSC_NAME:-}" ]]; then
    printf '%s' "$CSC_NAME"
    return
  fi

  local keychain_args=()
  if [[ -n "${KEYCHAIN_PATH:-}" ]]; then
    keychain_args=("$KEYCHAIN_PATH")
  fi

  if [[ ${#keychain_args[@]} -gt 0 ]]; then
    security find-identity -v -p codesigning "${keychain_args[@]}" 2>/dev/null \
      | sed -n 's/.*"\(Developer ID Application:.*\)".*/\1/p' \
      | head -n 1
  else
    security find-identity -v -p codesigning 2>/dev/null \
      | sed -n 's/.*"\(Developer ID Application:.*\)".*/\1/p' \
      | head -n 1
  fi
}

SIGNING_IDENTITY="$(resolve_signing_identity)"
if [[ -z "$SIGNING_IDENTITY" ]]; then
  if [[ "${CI:-}" == "true" ]]; then
    echo "No Developer ID Application signing identity found for CI native beta packaging." >&2
    exit 1
  fi

  SIGNING_IDENTITY="-"
  echo "No Developer ID identity found; using ad-hoc signing for local package."
fi

if [[ "$SIGNING_IDENTITY" == "-" ]]; then
  codesign --force --options runtime --sign - "$APP_BUNDLE"
else
  codesign --force --timestamp --options runtime --sign "$SIGNING_IDENTITY" "$APP_BUNDLE"
fi

codesign --verify --deep --strict --verbose=2 "$APP_BUNDLE"

if [[ "$SIGNING_IDENTITY" != "-" ]]; then
  if [[ -n "${APPLE_ID:-}" && -n "${APPLE_APP_SPECIFIC_PASSWORD:-}" && -n "${APPLE_TEAM_ID:-}" ]]; then
    NOTARY_DIR="$(mktemp -d "$DIST_DIR/notary.XXXXXX")"
    NOTARY_ZIP="$NOTARY_DIR/$APP_NAME-notary.zip"
    COPYFILE_DISABLE=1 ditto -c -k --norsrc --noextattr --keepParent "$APP_BUNDLE" "$NOTARY_ZIP"
    xcrun notarytool submit "$NOTARY_ZIP" \
      --apple-id "$APPLE_ID" \
      --password "$APPLE_APP_SPECIFIC_PASSWORD" \
      --team-id "$APPLE_TEAM_ID" \
      --wait
    rm -rf "$NOTARY_DIR"
    xcrun stapler staple "$APP_BUNDLE"
    xcrun stapler validate "$APP_BUNDLE"
  elif [[ "${CI:-}" == "true" ]]; then
    echo "Apple notarization credentials are required for CI native beta packaging." >&2
    exit 1
  fi
fi

rm -f "$VERSIONED_ZIP" "$ALIAS_ZIP"
COPYFILE_DISABLE=1 ditto -c -k --norsrc --noextattr --keepParent "$APP_BUNDLE" "$VERSIONED_ZIP"
cp "$VERSIONED_ZIP" "$ALIAS_ZIP"

echo "Created native Mac beta artifacts:"
echo "$VERSIONED_ZIP"
echo "$ALIAS_ZIP"
