#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT_DIR"

CARGO="${CARGO:-cargo}"
UNAME_S="$(uname -s)"
if [[ "$CARGO" == "cargo" && "$UNAME_S" != "Linux" ]]; then
  echo "GPUI Linux beta packaging must run on Linux." >&2
  exit 1
fi

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

DIST_DIR="${DIST_DIR:-$ROOT_DIR/dist/gpui-linux}"
VERSION="${VERSION:-}"
EXECUTABLE_NAME="MdowNative"
HOST_ARCH="${ARCH:-$(uname -m)}"
case "$HOST_ARCH" in
  x64 | x86_64 | amd64) ARCH_LABEL="x64" ;;
  aarch64 | arm64) ARCH_LABEL="arm64" ;;
  *)
    echo "Unsupported Linux architecture: $HOST_ARCH" >&2
    exit 1
    ;;
esac

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
  CREATED_TEMP_DIR="$(mktemp -d "$TEMP_BASE/mdow-gpui-linux.XXXXXX")"
  [[ -n "$CREATED_TEMP_DIR" && -d "$CREATED_TEMP_DIR" ]] || {
    echo "Failed to create packaging temporary directory." >&2
    exit 1
  }
  CREATED_TEMP_DIR="$(cd "$CREATED_TEMP_DIR" && pwd -P)"
  TEMP_PATHS+=("$CREATED_TEMP_DIR")
}

if [[ -z "$VERSION" ]]; then
  VERSION="$(node -p "require('$ROOT_DIR/apps/desktop/package.json').version")"
fi
if [[ -z "$VERSION" ]]; then
  echo "Unable to resolve GPUI Linux package version." >&2
  exit 1
fi

mkdir -p "$DIST_DIR"
DIST_DIR="$(cd "$DIST_DIR" && pwd -P)"
BUNDLE_NAME="MdowNative-$VERSION-$ARCH_LABEL-linux-beta"
STAGE_DIR="$DIST_DIR/$BUNDLE_NAME"
VERSIONED_ZIP="$DIST_DIR/$BUNDLE_NAME.zip"
ALIAS_ZIP="$DIST_DIR/MdowNative-linux-beta.zip"

TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT_DIR/apps/gpui/target}"
if [[ "$TARGET_DIR" != /* ]]; then
  TARGET_DIR="$ROOT_DIR/$TARGET_DIR"
fi
BUILD_BINARY="$TARGET_DIR/release/mdow-gpui"

echo "Building Mdow Native Linux release binary"
"$CARGO" build --release --locked --manifest-path "$ROOT_DIR/apps/gpui/Cargo.toml"
if [[ ! -x "$BUILD_BINARY" ]]; then
  echo "Cargo did not create the expected executable: $BUILD_BINARY" >&2
  exit 1
fi

if [[ -z "$STAGE_DIR" || "$(dirname "$STAGE_DIR")" != "$DIST_DIR" || "$(basename "$STAGE_DIR")" != "$BUNDLE_NAME" ]]; then
  echo "Refusing to replace unvalidated stage path: $STAGE_DIR" >&2
  exit 1
fi
rm -rf -- "$STAGE_DIR"
mkdir -p "$STAGE_DIR/assets"
cp "$BUILD_BINARY" "$STAGE_DIR/$EXECUTABLE_NAME"
chmod +x "$STAGE_DIR/$EXECUTABLE_NAME"
cp -R "$ROOT_DIR/apps/gpui/assets/." "$STAGE_DIR/assets/"

VERIFIED_ASSET_ROOT="$(
  cd "$STAGE_DIR"
  "./$EXECUTABLE_NAME" --verify-assets
)"
EXPECTED_ASSET_ROOT="$(cd "$STAGE_DIR/assets" && pwd -P)"
if [[ "$VERIFIED_ASSET_ROOT" != "$EXPECTED_ASSET_ROOT" ]]; then
  echo "Staged GPUI Linux beta reported asset root '$VERIFIED_ASSET_ROOT'; expected '$EXPECTED_ASSET_ROOT'." >&2
  exit 1
fi

rm -f -- "$VERSIONED_ZIP" "$ALIAS_ZIP"
(
  cd "$DIST_DIR"
  zip -qr "$VERSIONED_ZIP" "$BUNDLE_NAME"
)
cp "$VERSIONED_ZIP" "$ALIAS_ZIP"

make_temp_dir
VALIDATION_DIR="$CREATED_TEMP_DIR"
unzip -q "$VERSIONED_ZIP" -d "$VALIDATION_DIR"
EXTRACTED_DIR="$VALIDATION_DIR/$BUNDLE_NAME"
EXTRACTED_BINARY="$EXTRACTED_DIR/$EXECUTABLE_NAME"
if [[ ! -x "$EXTRACTED_BINARY" ]]; then
  echo "Extracted GPUI Linux beta is missing $EXECUTABLE_NAME." >&2
  exit 1
fi
if [[ ! -d "$EXTRACTED_DIR/assets/fonts" || ! -d "$EXTRACTED_DIR/assets/icons" ]]; then
  echo "Extracted GPUI Linux beta is missing bundled assets." >&2
  exit 1
fi
EXTRACTED_ASSET_ROOT="$(
  cd "$EXTRACTED_DIR"
  "./$EXECUTABLE_NAME" --verify-assets
)"
EXPECTED_EXTRACTED_ASSETS="$(cd "$EXTRACTED_DIR/assets" && pwd -P)"
if [[ "$EXTRACTED_ASSET_ROOT" != "$EXPECTED_EXTRACTED_ASSETS" ]]; then
  echo "Extracted GPUI Linux beta reported asset root '$EXTRACTED_ASSET_ROOT'; expected '$EXPECTED_EXTRACTED_ASSETS'." >&2
  exit 1
fi

echo "Created GPUI Linux beta artifacts:"
echo "$VERSIONED_ZIP"
echo "$ALIAS_ZIP"
