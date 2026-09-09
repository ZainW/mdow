#!/usr/bin/env bash
# Download and verify Sparkle 2 for the Mac Native (GPUI) updater.
# CI and local `cargo` builds on macOS call this from apps/gpui/build.rs.
# Not used on Linux.
set -euo pipefail

SPARKLE_VERSION="${SPARKLE_VERSION:-2.9.6}"
SPARKLE_SHA256="${SPARKLE_SHA256:-52bf9e88cdd972fc0c81501377a880e90d47031bd8ca5462488f843e2609e192}"
SPARKLE_URL="${SPARKLE_URL:-https://github.com/sparkle-project/Sparkle/releases/download/${SPARKLE_VERSION}/Sparkle-${SPARKLE_VERSION}.tar.xz}"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
VENDOR_DIR="${SPARKLE_VENDOR_DIR:-$ROOT_DIR/apps/gpui/vendor/Sparkle}"
FRAMEWORK="$VENDOR_DIR/Sparkle.framework"
GENERATE_APPCAST="$VENDOR_DIR/bin/generate_appcast"

if [[ -d "$FRAMEWORK" && -x "$GENERATE_APPCAST" && "${SPARKLE_FORCE_FETCH:-}" != "1" ]]; then
  exit 0
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required to fetch Sparkle $SPARKLE_VERSION." >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/mdow-sparkle.XXXXXX")"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

ARCHIVE="$TMP_DIR/Sparkle-${SPARKLE_VERSION}.tar.xz"
echo "Fetching Sparkle $SPARKLE_VERSION"
curl -fsSL "$SPARKLE_URL" -o "$ARCHIVE"

actual_sha=""
if command -v shasum >/dev/null 2>&1; then
  actual_sha="$(shasum -a 256 "$ARCHIVE" | awk '{ print $1 }')"
elif command -v sha256sum >/dev/null 2>&1; then
  actual_sha="$(sha256sum "$ARCHIVE" | awk '{ print $1 }')"
else
  echo "Need shasum or sha256sum to verify Sparkle." >&2
  exit 1
fi
if [[ "$actual_sha" != "$SPARKLE_SHA256" ]]; then
  echo "Sparkle checksum mismatch for $SPARKLE_URL" >&2
  echo "  expected: $SPARKLE_SHA256" >&2
  echo "  actual:   $actual_sha" >&2
  exit 1
fi

EXTRACT="$TMP_DIR/extract"
mkdir -p "$EXTRACT"
tar -xJf "$ARCHIVE" -C "$EXTRACT"

found_framework="$(find "$EXTRACT" -name Sparkle.framework -type d | head -n 1)"
found_bin="$(find "$EXTRACT" -path '*/bin/generate_appcast' -type f | head -n 1)"
if [[ -z "$found_framework" || -z "$found_bin" ]]; then
  echo "Sparkle archive did not contain Sparkle.framework and bin/generate_appcast." >&2
  exit 1
fi

rm -rf "$VENDOR_DIR"
mkdir -p "$VENDOR_DIR/bin"
# Preserve framework symlinks; do not follow them.
ditto_bin="${DITTO:-}"
if [[ -z "$ditto_bin" && "$(uname -s)" == "Darwin" ]]; then
  ditto_bin="ditto"
fi
if [[ -n "$ditto_bin" ]]; then
  "$ditto_bin" "$found_framework" "$FRAMEWORK"
else
  cp -a "$found_framework" "$FRAMEWORK"
fi
cp -a "$(dirname "$found_bin")/." "$VENDOR_DIR/bin/"
chmod +x "$VENDOR_DIR/bin/"* 2>/dev/null || true

if [[ ! -d "$FRAMEWORK" || ! -x "$GENERATE_APPCAST" ]]; then
  echo "Failed to vendor Sparkle into $VENDOR_DIR" >&2
  exit 1
fi

echo "Sparkle $SPARKLE_VERSION ready at $VENDOR_DIR"
