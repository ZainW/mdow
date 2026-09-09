#!/usr/bin/env bash
# Sign the Mac Native zip with Sparkle EdDSA and write appcast-native-mac.xml.
# Requires SPARKLE_PRIVATE_ED_KEY (GitHub Actions secret). Never commit that key.
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Native Sparkle appcast generation must run on macOS." >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
DIST_DIR="${DIST_DIR:-$ROOT_DIR/dist/gpui-mac}"
VERSION="${VERSION:-}"
REPO="${GITHUB_REPOSITORY:-ZainW/mdow}"
TAG="${GITHUB_REF_NAME:-}"
FEED_NAME="appcast-native-mac.xml"
DOWNLOAD_URL_PREFIX="${SPARKLE_DOWNLOAD_URL_PREFIX:-}"

if [[ -z "$VERSION" ]]; then
  echo "VERSION is required (marketing version, without a leading v)." >&2
  exit 1
fi

if [[ -z "$DOWNLOAD_URL_PREFIX" ]]; then
  if [[ -n "$TAG" ]]; then
    DOWNLOAD_URL_PREFIX="https://github.com/${REPO}/releases/download/${TAG}/"
  else
    DOWNLOAD_URL_PREFIX="https://github.com/${REPO}/releases/download/v${VERSION}/"
  fi
fi

VERSIONED_ZIP="$DIST_DIR/MdowNative-$VERSION-arm64-mac-beta.zip"
APPCAST_OUT="$DIST_DIR/$FEED_NAME"

if [[ ! -f "$VERSIONED_ZIP" ]]; then
  echo "Missing Native zip for appcast: $VERSIONED_ZIP" >&2
  exit 1
fi

if [[ -z "${SPARKLE_PRIVATE_ED_KEY:-}" ]]; then
  echo "SPARKLE_PRIVATE_ED_KEY is unset; skipping Native Sparkle appcast." >&2
  echo "Add the secret and re-run a release after generating keys (see apps/gpui/sparkle/README.md)." >&2
  rm -f "$APPCAST_OUT"
  exit 0
fi

bash "$ROOT_DIR/script/fetch_sparkle.sh"
GENERATE_APPCAST="${SPARKLE_VENDOR_DIR:-$ROOT_DIR/apps/gpui/vendor/Sparkle}/bin/generate_appcast"
if [[ ! -x "$GENERATE_APPCAST" ]]; then
  echo "Sparkle generate_appcast is missing after fetch." >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/mdow-sparkle-appcast.XXXXXX")"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

# generate_appcast signs every archive in the folder. Only the versioned zip
# is Sparkle's enclosure; the alias zip would duplicate the same bundle.
cp "$VERSIONED_ZIP" "$TMP_DIR/$(basename "$VERSIONED_ZIP")"

# Prefer stdin so the private key is never written to disk on the runner.
printf '%s' "$SPARKLE_PRIVATE_ED_KEY" | "$GENERATE_APPCAST" \
  --ed-key-file - \
  --download-url-prefix "$DOWNLOAD_URL_PREFIX" \
  --maximum-versions 1 \
  --maximum-deltas 0 \
  --link "https://github.com/${REPO}" \
  "$TMP_DIR"

generated="$(find "$TMP_DIR" -maxdepth 1 -name '*.xml' | head -n 1)"
if [[ -z "$generated" ]]; then
  echo "generate_appcast did not write an XML appcast." >&2
  exit 1
fi
cp "$generated" "$APPCAST_OUT"

if ! grep -F "MdowNative-$VERSION-arm64-mac-beta.zip" "$APPCAST_OUT" >/dev/null; then
  echo "Appcast enclosure is missing the Native zip filename." >&2
  exit 1
fi
if grep -F "latest-mac.yml" "$APPCAST_OUT" >/dev/null; then
  echo "Native appcast must not point at Electron latest-mac.yml." >&2
  exit 1
fi
if ! grep -F "sparkle:edSignature" "$APPCAST_OUT" >/dev/null; then
  echo "Appcast is missing sparkle:edSignature." >&2
  exit 1
fi

echo "Wrote $APPCAST_OUT"
echo "Feed URL: https://github.com/${REPO}/releases/latest/download/$FEED_NAME"
