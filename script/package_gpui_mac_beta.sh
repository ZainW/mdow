#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "GPUI Mac beta packaging must run on macOS." >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
source "$ROOT_DIR/script/native_mac_bundle.sh"
cd "$ROOT_DIR"

TEMP_BASE_INPUT="${TMPDIR:-/tmp}"
if [[ ! -d "$TEMP_BASE_INPUT" ]]; then
  echo "Temporary directory base does not exist: $TEMP_BASE_INPUT" >&2
  exit 1
fi
TEMP_BASE="$(cd "$TEMP_BASE_INPUT" && pwd -P)"
case "$TEMP_BASE" in
  "$ROOT_DIR" | "$ROOT_DIR"/*)
    echo "Temporary directory base must be outside repository: $TEMP_BASE" >&2
    exit 1
    ;;
esac

APP_BUNDLE_NAME="Mdow Native"
EXECUTABLE_NAME="MdowNative"
BUNDLE_ID="com.zain.mdow.gpui"
MIN_SYSTEM_VERSION="14.0"
DIST_DIR="${DIST_DIR:-$ROOT_DIR/dist/gpui-mac}"
VERSION="${VERSION:-}"
BUILD_NUMBER="${GITHUB_RUN_NUMBER:-0}"
CARGO="${CARGO:-cargo}"
CODESIGN="${CODESIGN:-codesign}"
DITTO="${DITTO:-ditto}"
LIPO="${LIPO:-lipo}"
XCRUN="${XCRUN:-xcrun}"
SPCTL="${SPCTL:-spctl}"
RUNNER_ARCH="${ARCH:-$(uname -m)}"
TEMP_PATHS=()
CREATED_TEMP_DIR=""

cleanup_temp_paths() {
  local temp_path
  for temp_path in "${TEMP_PATHS[@]}"; do
    if [[ -n "$temp_path" && -d "$temp_path" && "$(basename "$temp_path")" == mdow-gpui-* ]]; then
      rm -rf -- "$temp_path"
    else
      echo "Refusing to clean unvalidated temporary path: $temp_path" >&2
    fi
  done
}
trap cleanup_temp_paths EXIT

make_temp_dir() {
  CREATED_TEMP_DIR="$(mktemp -d "$TEMP_BASE/mdow-gpui-package.XXXXXX")"
  [[ -n "$CREATED_TEMP_DIR" && -d "$CREATED_TEMP_DIR" ]] || {
    echo "Failed to create packaging temporary directory." >&2
    exit 1
  }
  CREATED_TEMP_DIR="$(cd "$CREATED_TEMP_DIR" && pwd -P)"
  TEMP_PATHS+=("$CREATED_TEMP_DIR")
}

if [[ "$RUNNER_ARCH" != "arm64" ]]; then
  echo "GPUI Mac beta packaging requires an arm64 runner, got: $RUNNER_ARCH" >&2
  exit 1
fi

if [[ -z "$VERSION" ]]; then
  VERSION="$(awk -F '=' '
    /^version[[:space:]]*=/ {
      value = $2
      gsub(/^[[:space:]\"]+|[[:space:]\"]+$/, "", value)
      print value
      exit
    }
  ' "$ROOT_DIR/apps/gpui/Cargo.toml")"
fi
if [[ -z "$VERSION" ]]; then
  echo "Unable to resolve GPUI package version." >&2
  exit 1
fi

mkdir -p "$DIST_DIR"
DIST_DIR="$(cd "$DIST_DIR" && pwd -P)"
APP_BUNDLE="$DIST_DIR/$APP_BUNDLE_NAME.app"
APP_CONTENTS="$APP_BUNDLE/Contents"
APP_MACOS="$APP_CONTENTS/MacOS"
APP_BINARY="$APP_MACOS/$EXECUTABLE_NAME"
INFO_PLIST="$APP_CONTENTS/Info.plist"
VERSIONED_ZIP="$DIST_DIR/MdowNative-$VERSION-arm64-mac-beta.zip"
ALIAS_ZIP="$DIST_DIR/MdowNative-mac-beta.zip"

TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT_DIR/apps/gpui/target}"
if [[ "$TARGET_DIR" != /* ]]; then
  TARGET_DIR="$ROOT_DIR/$TARGET_DIR"
fi
BUILD_BINARY="$TARGET_DIR/release/mdow-gpui"

echo "Building $APP_BUNDLE_NAME release binary"
"$CARGO" build --release --locked --manifest-path "$ROOT_DIR/apps/gpui/Cargo.toml"
if [[ ! -x "$BUILD_BINARY" ]]; then
  echo "Cargo did not create the expected executable: $BUILD_BINARY" >&2
  exit 1
fi

BUILD_ARCHS="$("$LIPO" -archs "$BUILD_BINARY")"
if [[ "$BUILD_ARCHS" != "arm64" ]]; then
  echo "GPUI Mac beta binary must be arm64-only, got: $BUILD_ARCHS" >&2
  exit 1
fi

if [[ -z "$APP_BUNDLE" || "$(dirname "$APP_BUNDLE")" != "$DIST_DIR" || "$(basename "$APP_BUNDLE")" != "$APP_BUNDLE_NAME.app" ]]; then
  echo "Refusing to replace unvalidated app bundle path: $APP_BUNDLE" >&2
  exit 1
fi
rm -rf -- "$APP_BUNDLE"
mkdir -p "$APP_MACOS"
copy_native_mac_resources "$ROOT_DIR" "$APP_CONTENTS" "$EXECUTABLE_NAME"
cp -R "$ROOT_DIR/apps/gpui/assets" "$APP_CONTENTS/Resources/assets"
cp "$BUILD_BINARY" "$APP_BINARY"
chmod +x "$APP_BINARY"

write_native_mac_info_plist \
  "$INFO_PLIST" \
  "$EXECUTABLE_NAME" \
  "$APP_BUNDLE_NAME" \
  "$BUNDLE_ID" \
  "$MIN_SYSTEM_VERSION" \
  "$VERSION" \
  "$BUILD_NUMBER"

resolve_signing_identity() {
  if [[ -n "${NATIVE_MAC_CODESIGN_IDENTITY:-}" ]]; then
    printf '%s\n' "$NATIVE_MAC_CODESIGN_IDENTITY"
    return
  fi

  if [[ -n "${CSC_NAME:-}" ]]; then
    printf '%s\n' "$CSC_NAME"
    return
  fi

  if [[ -n "${KEYCHAIN_PATH:-}" ]]; then
    /usr/bin/security find-identity -v -p codesigning "$KEYCHAIN_PATH" 2>/dev/null \
      | sed -n 's/.*"\(Developer ID Application:.*\)".*/\1/p' \
      | head -n 1
  fi
}

SIGNING_IDENTITY="$(resolve_signing_identity)"
if [[ "${CI:-}" == "true" && ( -z "$SIGNING_IDENTITY" || "$SIGNING_IDENTITY" == "-" ) ]]; then
  echo "A real Developer ID Application signing identity is required for CI GPUI beta packaging." >&2
  exit 1
fi
if [[ -z "$SIGNING_IDENTITY" ]]; then
  SIGNING_IDENTITY="-"
  echo "No Developer ID identity found; using ad-hoc signing for local package."
fi

if [[ "$SIGNING_IDENTITY" == "-" ]]; then
  "$CODESIGN" --force --options runtime --sign - "$APP_BUNDLE"
else
  "$CODESIGN" --force --timestamp --options runtime --sign "$SIGNING_IDENTITY" "$APP_BUNDLE"
fi
"$CODESIGN" --verify --deep --strict --verbose=2 "$APP_BUNDLE"

SIGNED_WITH_DEVELOPER_ID=false
if [[ "$SIGNING_IDENTITY" != "-" ]]; then
  SIGNATURE_DETAILS="$("$CODESIGN" -dv --verbose=4 "$APP_BUNDLE" 2>&1)"
  while IFS= read -r signature_detail; do
    if [[ "$signature_detail" == "Authority=Developer ID Application:"* ]]; then
      SIGNED_WITH_DEVELOPER_ID=true
      break
    fi
  done <<<"$SIGNATURE_DETAILS"
fi
if [[ "${CI:-}" == "true" && "$SIGNED_WITH_DEVELOPER_ID" != "true" ]]; then
  echo "CI GPUI beta packaging requires a verified Developer ID Application signature." >&2
  exit 1
fi

DID_NOTARIZE=false
if [[ "$SIGNED_WITH_DEVELOPER_ID" == "true" ]]; then
  if [[ -n "${APPLE_ID:-}" && -n "${APPLE_APP_SPECIFIC_PASSWORD:-}" && -n "${APPLE_TEAM_ID:-}" ]]; then
    make_temp_dir
    NOTARY_DIR="$CREATED_TEMP_DIR"
    NOTARY_ZIP="$NOTARY_DIR/MdowNative-notary.zip"
    COPYFILE_DISABLE=1 "$DITTO" -c -k --norsrc --noextattr --keepParent \
      "$APP_BUNDLE" "$NOTARY_ZIP"
    "$XCRUN" notarytool submit "$NOTARY_ZIP" \
      --apple-id "$APPLE_ID" \
      --password "$APPLE_APP_SPECIFIC_PASSWORD" \
      --team-id "$APPLE_TEAM_ID" \
      --wait
    "$XCRUN" stapler staple "$APP_BUNDLE"
    "$XCRUN" stapler validate "$APP_BUNDLE"
    DID_NOTARIZE=true
  elif [[ "${CI:-}" == "true" ]]; then
    echo "Apple notarization credentials are required for CI GPUI beta packaging." >&2
    exit 1
  else
    echo "No Apple notarization credentials found; skipping local notarization."
  fi
fi

rm -f -- "$VERSIONED_ZIP" "$ALIAS_ZIP"
COPYFILE_DISABLE=1 "$DITTO" -c -k --norsrc --noextattr --keepParent \
  "$APP_BUNDLE" "$VERSIONED_ZIP"
cp "$VERSIONED_ZIP" "$ALIAS_ZIP"

make_temp_dir
VALIDATION_DIR="$CREATED_TEMP_DIR"
"$DITTO" -x -k "$VERSIONED_ZIP" "$VALIDATION_DIR"
EXTRACTED_APP="$VALIDATION_DIR/$APP_BUNDLE_NAME.app"
EXTRACTED_BINARY="$EXTRACTED_APP/Contents/MacOS/$EXECUTABLE_NAME"
"$CODESIGN" --verify --deep --strict --verbose=2 "$EXTRACTED_APP"
EXTRACTED_ARCHS="$("$LIPO" -archs "$EXTRACTED_BINARY")"
if [[ "$EXTRACTED_ARCHS" != "arm64" ]]; then
  echo "Extracted GPUI Mac beta binary must be arm64-only, got: $EXTRACTED_ARCHS" >&2
  exit 1
fi
if [[ "$DID_NOTARIZE" == "true" ]]; then
  "$SPCTL" -a -vv --type execute "$EXTRACTED_APP"
  "$XCRUN" stapler validate "$EXTRACTED_APP"
fi
(
  cd "$VALIDATION_DIR"
  "$EXTRACTED_BINARY" --verify-assets
)

echo "Created GPUI Mac beta artifacts:"
echo "$VERSIONED_ZIP"
echo "$ALIAS_ZIP"
