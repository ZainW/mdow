#!/bin/bash
# Patch Electron.app's Info.plist so macOS shows "Mdow" in the menu bar
# and dock tooltip during development. Runs as a postinstall hook.

APP="node_modules/electron/dist/Electron.app"
PLIST="$APP/Contents/Info.plist"

if [ ! -f "$PLIST" ]; then
  exit 0
fi

/usr/libexec/PlistBuddy -c "Set :CFBundleName Mdow" "$PLIST" 2>/dev/null
/usr/libexec/PlistBuddy -c "Set :CFBundleDisplayName Mdow" "$PLIST" 2>/dev/null

# Re-sign so macOS trusts the modified plist
codesign --force --deep --sign - "$APP" 2>/dev/null

echo "Patched Electron.app name to Mdow"
