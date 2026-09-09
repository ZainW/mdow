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

sparkle_header_path() {
  local fw="$1"
  local header
  for header in \
    "$fw/Headers/Sparkle.h" \
    "$fw/Versions/Current/Headers/Sparkle.h" \
    "$fw/Versions/B/Headers/Sparkle.h"; do
    if [[ -f "$header" ]]; then
      printf '%s\n' "$header"
      return 0
    fi
  done
  return 1
}

# The Sparkle tarball ships Sparkle.framework at the root (with Headers) and a
# nested copy inside Sparkle Test App.app that has no public headers. Never
# take `find | head` — on macOS that nested copy is often first.
pick_sparkle_framework() {
  local extract="$1"
  local candidate
  if sparkle_header_path "$extract/Sparkle.framework" >/dev/null; then
    printf '%s\n' "$extract/Sparkle.framework"
    return 0
  fi
  while IFS= read -r -d '' candidate; do
    case "$candidate" in
      *.app/*) continue ;;
    esac
    if sparkle_header_path "$candidate" >/dev/null; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done < <(find "$extract" -name Sparkle.framework -type d -print0)
  return 1
}

pick_generate_appcast() {
  local extract="$1"
  if [[ -x "$extract/bin/generate_appcast" ]]; then
    printf '%s\n' "$extract/bin/generate_appcast"
    return 0
  fi
  local candidate
  while IFS= read -r -d '' candidate; do
    case "$candidate" in
      *.app/*) continue ;;
    esac
    if [[ -f "$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done < <(find "$extract" -path '*/bin/generate_appcast' -type f -print0)
  return 1
}

self_test() {
  local tmp
  tmp="$(mktemp -d "${TMPDIR:-/tmp}/mdow-sparkle-self-test.XXXXXX")"
  trap 'rm -rf "$tmp"' RETURN

  local nested="$tmp/Sparkle Test App.app/Contents/Frameworks/Sparkle.framework"
  mkdir -p "$nested/Versions/B"
  printf 'nested\n' >"$nested/Sparkle"
  mkdir -p "$tmp/Sparkle.framework/Headers"
  printf '// public\n' >"$tmp/Sparkle.framework/Headers/Sparkle.h"
  mkdir -p "$tmp/bin"
  printf '#!/bin/sh\n' >"$tmp/bin/generate_appcast"
  chmod +x "$tmp/bin/generate_appcast"

  local picked
  picked="$(pick_sparkle_framework "$tmp")"
  if [[ "$picked" != "$tmp/Sparkle.framework" ]]; then
    echo "self-test: expected root Sparkle.framework, got $picked" >&2
    exit 1
  fi
  if ! sparkle_header_path "$picked" >/dev/null; then
    echo "self-test: picked framework is missing Sparkle.h" >&2
    exit 1
  fi
  local bin
  bin="$(pick_generate_appcast "$tmp")"
  if [[ "$bin" != "$tmp/bin/generate_appcast" ]]; then
    echo "self-test: expected root generate_appcast, got $bin" >&2
    exit 1
  fi
  echo "fetch_sparkle.sh self-test ok"
}

if [[ "${1:-}" == "--self-test" ]]; then
  self_test
  exit 0
fi

if [[ -d "$FRAMEWORK" && -x "$GENERATE_APPCAST" && "${SPARKLE_FORCE_FETCH:-}" != "1" ]]; then
  if sparkle_header_path "$FRAMEWORK" >/dev/null; then
    exit 0
  fi
  echo "Existing Sparkle.framework is missing public headers; re-fetching." >&2
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

found_framework="$(pick_sparkle_framework "$EXTRACT" || true)"
found_bin="$(pick_generate_appcast "$EXTRACT" || true)"
if [[ -z "$found_framework" || -z "$found_bin" ]]; then
  echo "Sparkle archive did not contain a headered Sparkle.framework and bin/generate_appcast." >&2
  find "$EXTRACT" -name Sparkle.framework -type d -print >&2 || true
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
if ! sparkle_header_path "$FRAMEWORK" >/dev/null; then
  echo "Vendored Sparkle.framework is missing Headers/Sparkle.h (picked $found_framework)." >&2
  exit 1
fi

echo "Sparkle $SPARKLE_VERSION ready at $VENDOR_DIR"
