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

write_native_mac_info_plist() {
  local plist_path="$1"
  local app_name="$2"
  local bundle_id="$3"
  local min_system_version="$4"
  local version="${5:-}"
  local build_number="${6:-}"

  {
    cat <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key>
  <string>$app_name</string>
  <key>CFBundleIdentifier</key>
  <string>$bundle_id</string>
  <key>CFBundleName</key>
  <string>$app_name</string>
  <key>CFBundleDisplayName</key>
  <string>Mdow</string>
  <key>CFBundleIconFile</key>
  <string>$app_name.icns</string>
  <key>CFBundleIconName</key>
  <string>$app_name</string>
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
