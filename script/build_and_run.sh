#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-run}"
if [[ "$MODE" != "run" && "$MODE" != "debug" && "$MODE" != "--debug" && "$MODE" != "logs" && "$MODE" != "--logs" && "$MODE" != "telemetry" && "$MODE" != "--telemetry" && "$MODE" != "verify" && "$MODE" != "--verify" ]]; then
  MODE="run"
fi
APP_ARGS=()
if [[ "${1:-}" == "$MODE" ]]; then
  shift || true
fi
APP_ARGS=("$@")
APP_NAME="MdowNative"
BUNDLE_ID="com.zain.mdow.native"
MIN_SYSTEM_VERSION="14.0"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_DIR="$ROOT_DIR/apps/native-mac"
DIST_DIR="$ROOT_DIR/dist"
APP_BUNDLE="$DIST_DIR/$APP_NAME.app"
APP_CONTENTS="$APP_BUNDLE/Contents"
APP_MACOS="$APP_CONTENTS/MacOS"
APP_BINARY="$APP_MACOS/$APP_NAME"
INFO_PLIST="$APP_CONTENTS/Info.plist"

pkill -x "$APP_NAME" >/dev/null 2>&1 || true
for _ in {1..50}; do
  if ! pgrep -x "$APP_NAME" >/dev/null 2>&1; then
    break
  fi
  sleep 0.1
done

swift build --package-path "$APP_DIR"
BUILD_BINARY="$(swift build --package-path "$APP_DIR" --show-bin-path)/$APP_NAME"

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

prepare_launch_state() {
  rm -rf "$HOME/Library/Saved Application State/$BUNDLE_ID.savedState"
  defaults delete "$BUNDLE_ID" ApplePersistenceIgnoreState >/dev/null 2>&1 || true
  while IFS= read -r default_key; do
    defaults delete "$BUNDLE_ID" "$default_key" >/dev/null 2>&1 || true
  done < <(
    defaults read "$BUNDLE_ID" 2>/dev/null \
      | sed -n 's/^    "\\(NSWindow Frame .*\\)" = .*/\\1/p; s/^    "\\(NSSplitView Subview Frames .*\\)" = .*/\\1/p'
  )
}

visible_app_window_count() {
  APP_NAME_FOR_SWIFT="$APP_NAME" /usr/bin/swift -e 'import CoreGraphics
import Foundation

let appName = ProcessInfo.processInfo.environment["APP_NAME_FOR_SWIFT"] ?? ""
let windows = (CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]]) ?? []
let matchingWindows = windows.filter { window in
  let ownerName = window[kCGWindowOwnerName as String] as? String
  let layer = window[kCGWindowLayer as String] as? Int
  let alpha = window[kCGWindowAlpha as String] as? Double
  return ownerName == appName && layer == 0 && (alpha ?? 0) > 0
}

print(matchingWindows.count)'
}

open_app() {
  prepare_launch_state

  if [[ ${#APP_ARGS[@]} -gt 0 ]]; then
    HAS_DIRECTORY=false
    for app_arg in "${APP_ARGS[@]}"; do
      if [[ -d "$app_arg" ]]; then
        HAS_DIRECTORY=true
        break
      fi
    done

    if [[ "$HAS_DIRECTORY" == true ]]; then
      /usr/bin/open -n "$APP_BUNDLE" --args "${APP_ARGS[@]}"
    else
      /usr/bin/open -n -a "$APP_BUNDLE" "${APP_ARGS[@]}"
    fi
  else
    /usr/bin/open -n "$APP_BUNDLE"
  fi
}

case "$MODE" in
  run)
    open_app
    ;;
  --debug|debug)
    lldb -- "$APP_BINARY"
    ;;
  --logs|logs)
    open_app
    /usr/bin/log stream --info --style compact --predicate "process == \"$APP_NAME\""
    ;;
  --telemetry|telemetry)
    open_app
    /usr/bin/log stream --info --style compact --predicate "subsystem == \"$BUNDLE_ID\""
    ;;
  --verify|verify)
    VERIFY_BASE=""
    if [[ ${#APP_ARGS[@]} -gt 0 ]]; then
      VERIFY_FILE="${APP_ARGS[0]}"
    else
      VERIFY_BASE="$(mktemp "$ROOT_DIR/.mdow-native-verify.XXXXXX")"
      VERIFY_FILE="$VERIFY_BASE.md"
      printf '# Native verify\n\nMdowNative opened this Markdown file.\n' >"$VERIFY_FILE"
    fi
    prepare_launch_state
    /usr/bin/open -n -a "$APP_BUNDLE" "$VERIFY_FILE"
    sleep 2
    pgrep -x "$APP_NAME" >/dev/null
    WINDOW_COUNT="$(visible_app_window_count)"
    if [[ "$WINDOW_COUNT" -eq 0 ]]; then
      echo "$APP_NAME launched without opening a window" >&2
      exit 1
    fi
    if [[ "$WINDOW_COUNT" -gt 1 ]]; then
      echo "$APP_NAME launched with duplicate windows: $WINDOW_COUNT" >&2
      exit 1
    fi
    sleep 6
    WINDOW_COUNT="$(visible_app_window_count)"
    if [[ "$WINDOW_COUNT" -eq 0 ]]; then
      echo "$APP_NAME window did not remain open" >&2
      exit 1
    fi
    if [[ "$WINDOW_COUNT" -gt 1 ]]; then
      echo "$APP_NAME kept duplicate windows open: $WINDOW_COUNT" >&2
      exit 1
    fi
    if [[ -n "$VERIFY_BASE" ]]; then
      rm -f "$VERIFY_FILE" "$VERIFY_BASE"
    fi
    ;;
  *)
    echo "usage: $0 [run|--debug|--logs|--telemetry|--verify]" >&2
    exit 2
    ;;
esac
