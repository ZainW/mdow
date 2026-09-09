#!/usr/bin/env bash

copy_native_mac_resources() {
  local root_dir="$1"
  local app_contents="$2"
  local app_name="$3"
  local resources_dir="$app_contents/Resources"
  local icon_source="$root_dir/apps/desktop/resources/icon.icns"

  mkdir -p "$resources_dir"
  cp "$icon_source" "$resources_dir/$app_name.icns"
}

read_sparkle_public_ed_key() {
  local key_file="$1"
  if [[ -n "${SPARKLE_ED_PUBLIC_KEY:-}" ]]; then
    printf '%s' "$SPARKLE_ED_PUBLIC_KEY" | tr -d '[:space:]'
    return
  fi
  if [[ ! -f "$key_file" ]]; then
    return 0
  fi
  awk 'NF && $1 !~ /^#/' "$key_file" | head -n 1 | tr -d '[:space:]'
}

sparkle_feed_url() {
  printf '%s\n' "${SPARKLE_FEED_URL:-https://github.com/ZainW/mdow/releases/latest/download/appcast-native-mac.xml}"
}

copy_sparkle_framework() {
  local source_framework="$1"
  local app_contents="$2"
  local ditto_bin="${3:-ditto}"
  local frameworks_dir="$app_contents/Frameworks"
  local dest="$frameworks_dir/Sparkle.framework"

  if [[ ! -d "$source_framework" ]]; then
    echo "Sparkle.framework not found at $source_framework" >&2
    return 1
  fi
  mkdir -p "$frameworks_dir"
  rm -rf -- "$dest"
  "$ditto_bin" "$source_framework" "$dest"
  rm -rf -- "$dest/Versions/B/XPCServices" "$dest/Versions/Current/XPCServices"
}

sign_sparkle_framework() {
  local framework="$1"
  local identity="$2"
  local codesign_bin="${3:-codesign}"
  local inner="$framework/Versions/Current"
  if [[ ! -d "$inner" ]]; then
    inner="$framework/Versions/B"
  fi

  local sign_args=(--force --options runtime --sign "$identity")
  if [[ "$identity" != "-" ]]; then
    sign_args=(--force --timestamp --options runtime --sign "$identity")
  fi

  if [[ -e "$inner/Autoupdate" ]]; then
    if [[ "$identity" != "-" ]]; then
      "$codesign_bin" --force --timestamp --options runtime --preserve-metadata=entitlements \
        --sign "$identity" "$inner/Autoupdate"
    else
      "$codesign_bin" --force --options runtime --sign - "$inner/Autoupdate"
    fi
  fi
  if [[ -d "$inner/Updater.app" ]]; then
    "$codesign_bin" "${sign_args[@]}" "$inner/Updater.app"
  fi
  "$codesign_bin" "${sign_args[@]}" "$framework"
}

write_native_mac_info_plist() {
  local plist_path="$1"
  local executable_name="$2"
  local display_name="$3"
  local bundle_id="$4"
  local min_system_version="$5"
  local version="${6:-}"
  local build_number="${7:-}"
  local sparkle_feed="${8:-}"
  local sparkle_public_ed_key="${9:-}"

  {
    cat <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key>
  <string>$executable_name</string>
  <key>CFBundleIdentifier</key>
  <string>$bundle_id</string>
  <key>CFBundleName</key>
  <string>$display_name</string>
  <key>CFBundleDisplayName</key>
  <string>$display_name</string>
  <key>CFBundleIconFile</key>
  <string>$executable_name.icns</string>
  <key>CFBundleIconName</key>
  <string>$executable_name</string>
PLIST

    if [[ -n "$version" ]]; then
      cat <<PLIST
  <key>CFBundleShortVersionString</key>
  <string>$version</string>
PLIST
    fi

    if [[ -n "$build_number" ]]; then
      cat <<PLIST
  <key>CFBundleVersion</key>
  <string>$build_number</string>
PLIST
    fi

    cat <<PLIST
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.developer-tools</string>
  <key>LSMinimumSystemVersion</key>
  <string>$min_system_version</string>
  <key>NSPrincipalClass</key>
  <string>NSApplication</string>
  <key>NSQuitAlwaysKeepsWindows</key>
  <false/>
PLIST

    if [[ -n "$sparkle_public_ed_key" ]]; then
      cat <<PLIST
  <key>SUFeedURL</key>
  <string>$sparkle_feed</string>
  <key>SUPublicEDKey</key>
  <string>$sparkle_public_ed_key</string>
  <key>SUEnableAutomaticChecks</key>
  <true/>
  <key>SUAutomaticallyUpdate</key>
  <false/>
  <key>SUShowReleaseNotes</key>
  <false/>
  <key>SUScheduledCheckInterval</key>
  <integer>86400</integer>
PLIST
    fi

    cat <<PLIST
  <key>CFBundleDocumentTypes</key>
  <array>
    <dict>
      <key>CFBundleTypeName</key>
      <string>Markdown Document</string>
      <key>CFBundleTypeRole</key>
      <string>Viewer</string>
      <key>LSHandlerRank</key>
      <string>Alternate</string>
      <key>LSItemContentTypes</key>
      <array>
        <string>net.daringfireball.markdown</string>
      </array>
      <key>CFBundleTypeExtensions</key>
      <array>
        <string>md</string>
        <string>markdown</string>
        <string>mdx</string>
      </array>
    </dict>
  </array>
  <key>UTImportedTypeDeclarations</key>
  <array>
    <dict>
      <key>UTTypeIdentifier</key>
      <string>net.daringfireball.markdown</string>
      <key>UTTypeDescription</key>
      <string>Markdown Document</string>
      <key>UTTypeConformsTo</key>
      <array>
        <string>public.plain-text</string>
      </array>
      <key>UTTypeTagSpecification</key>
      <dict>
        <key>public.filename-extension</key>
        <array>
          <string>md</string>
          <string>markdown</string>
          <string>mdx</string>
        </array>
      </dict>
    </dict>
  </array>
</dict>
</plist>
PLIST
  } >"$plist_path"
}
