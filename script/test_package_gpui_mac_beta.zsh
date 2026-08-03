#!/usr/bin/env zsh
set -euo pipefail

ROOT_DIR="${0:A:h:h}"
source "$ROOT_DIR/script/native_mac_bundle.sh"

test_dir="$(mktemp -d "${TMPDIR:-/tmp}/mdow-gpui-package-test.XXXXXX")"
cleanup() {
  [[ -n "$test_dir" && -d "$test_dir" ]] && rm -rf "$test_dir"
}
trap cleanup EXIT

plist="$test_dir/Info.plist"
write_native_mac_info_plist \
  "$plist" \
  "MdowNative" \
  "Mdow Native" \
  "com.zain.mdow.gpui" \
  "14.0" \
  "1.2.3" \
  "456"

[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$plist")" == "MdowNative" ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleDisplayName' "$plist")" == "Mdow Native" ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleName' "$plist")" == "Mdow Native" ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$plist")" == "com.zain.mdow.gpui" ]]

print "PASS: native mac plist supports distinct executable and display names"

PACKAGER="$ROOT_DIR/script/package_gpui_mac_beta.sh"

fail() {
  print -u2 -- "FAIL: $*"
  return 1
}

assert_file() {
  [[ -f "$1" ]] || fail "expected file: $1"
}

assert_dir() {
  [[ -d "$1" ]] || fail "expected directory: $1"
}

assert_contains() {
  local file="$1"
  local expected="$2"
  grep -F -- "$expected" "$file" >/dev/null || fail "expected '$expected' in $file"
}

assert_not_contains() {
  local file="$1"
  local unexpected="$2"
  if grep -F -- "$unexpected" "$file" >/dev/null; then
    fail "did not expect '$unexpected' in $file"
  fi
}

make_fakes() {
  local fake_dir="$1"
  mkdir -p "$fake_dir"

  cat >"$fake_dir/cargo" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$FAKE_TOOL_LOG/cargo"
mkdir -p "$CARGO_TARGET_DIR/release"
cat >"$CARGO_TARGET_DIR/release/mdow-gpui" <<'BINARY'
#!/usr/bin/env bash
set -euo pipefail
[[ "${1:-}" == "--verify-assets" ]]
case "$PWD/" in
  "$FAKE_REPO_ROOT/"*)
    echo "asset verification ran inside repository: $PWD" >&2
    exit 1
    ;;
esac
if [[ -n "${FAKE_VERIFY_ASSET_ROOT:-}" ]]; then
  asset_root="$FAKE_VERIFY_ASSET_ROOT"
else
  asset_root="$(cd "$(dirname "$0")/../Resources/assets" && pwd -P)"
  [[ -d "$asset_root/fonts" ]]
  [[ -d "$asset_root/icons" ]]
fi
printf '%s\n' "$asset_root" | tee "$FAKE_TOOL_LOG/verify-assets"
BINARY
chmod +x "$CARGO_TARGET_DIR/release/mdow-gpui"
FAKE

  cat >"$fake_dir/codesign" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$FAKE_TOOL_LOG/codesign"
if [[ "$*" == *"-dv"* && -n "${FAKE_CODESIGN_AUTHORITY:-}" ]]; then
  printf 'Authority=%s\n' "$FAKE_CODESIGN_AUTHORITY" >&2
fi
FAKE

  cat >"$fake_dir/ditto" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$FAKE_TOOL_LOG/ditto"
/usr/bin/ditto "$@"
if [[ "${1:-}" == "-x" && -n "${FAKE_EXTRACTED_PLIST_KEY:-}" ]]; then
  destination="${!#}"
  extracted_plist="$destination/Mdow Native.app/Contents/Info.plist"
  /usr/libexec/PlistBuddy \
    -c "Set :$FAKE_EXTRACTED_PLIST_KEY $FAKE_EXTRACTED_PLIST_VALUE" \
    "$extracted_plist"
fi
FAKE

  cat >"$fake_dir/lipo" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$FAKE_TOOL_LOG/lipo"
printf '%s\n' "$FAKE_LIPO_ARCHS"
FAKE

  cat >"$fake_dir/xcrun" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$FAKE_TOOL_LOG/xcrun"
FAKE

  cat >"$fake_dir/spctl" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$FAKE_TOOL_LOG/spctl"
FAKE

  chmod +x "$fake_dir"/*
}

run_packager() {
  local case_dir="$1"
  local fake_archs="$2"
  shift 2

  mkdir -p "$case_dir/log" "$case_dir/target" "$case_dir/dist" "$case_dir/tmp"
  make_fakes "$case_dir/fakes"

  env \
    -u APPLE_ID \
    -u APPLE_APP_SPECIFIC_PASSWORD \
    -u APPLE_TEAM_ID \
    -u CI \
    -u CSC_NAME \
    -u KEYCHAIN_PATH \
    -u NATIVE_MAC_CODESIGN_IDENTITY \
    ARCH=arm64 \
    VERSION=1.2.3 \
    GITHUB_RUN_NUMBER=456 \
    DIST_DIR="$case_dir/dist" \
    CARGO_TARGET_DIR="$case_dir/target" \
    TMPDIR="$case_dir/tmp" \
    FAKE_TOOL_LOG="$case_dir/log" \
    FAKE_REPO_ROOT="$ROOT_DIR" \
    FAKE_LIPO_ARCHS="$fake_archs" \
    "FAKE_CODESIGN_AUTHORITY=Developer ID Application: Test (TEAM123)" \
    CARGO="$case_dir/fakes/cargo" \
    CODESIGN="$case_dir/fakes/codesign" \
    DITTO="$case_dir/fakes/ditto" \
    LIPO="$case_dir/fakes/lipo" \
    XCRUN="$case_dir/fakes/xcrun" \
    SPCTL="$case_dir/fakes/spctl" \
    "$@" \
    bash "$PACKAGER"
}

[[ -f "$PACKAGER" ]] || fail "missing GPUI packager: $PACKAGER"
node - "$ROOT_DIR/package.json" <<'JS'
const fs = require('node:fs')

const packageJson = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'))
const scripts = packageJson.scripts ?? {}
if (scripts['package:gpui-mac-beta'] !== 'bash script/package_gpui_mac_beta.sh') process.exit(1)
if (scripts['test:package:gpui-mac-beta'] !== 'zsh script/test_package_gpui_mac_beta.zsh') {
  process.exit(1)
}
if ('package:native-mac-beta' in scripts) process.exit(1)
JS
[[ ! -e "$ROOT_DIR/script/package_native_mac_beta.sh" ]] || \
  fail "old Swift beta packager still exists"

repo_tmp_case="$test_dir/repo-tmp"
if run_packager "$repo_tmp_case" arm64 TMPDIR=. >"$repo_tmp_case.output" 2>&1; then
  fail "repository-contained TMPDIR unexpectedly packaged"
fi
assert_contains "$repo_tmp_case.output" "Temporary directory base must be outside repository"
[[ ! -s "$repo_tmp_case/log/cargo" ]] || fail "repository-contained TMPDIR reached Cargo"
print "PASS: repository-contained relative TMPDIR fails before building"

local_case="$test_dir/local"
run_packager "$local_case" arm64 >"$local_case.output" 2>&1

app="$local_case/dist/Mdow Native.app"
binary="$app/Contents/MacOS/MdowNative"
info_plist="$app/Contents/Info.plist"
[[ -x "$binary" ]] || fail "expected executable: $binary"
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$info_plist")" == "MdowNative" ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleDisplayName' "$info_plist")" == "Mdow Native" ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleName' "$info_plist")" == "Mdow Native" ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$info_plist")" == "com.zain.mdow.gpui" ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :LSMinimumSystemVersion' "$info_plist")" == "14.0" ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$info_plist")" == "1.2.3" ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' "$info_plist")" == "456" ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleDocumentTypes:0:LSHandlerRank' "$info_plist")" == "Alternate" ]]
assert_file "$app/Contents/Resources/MdowNative.icns"
assert_dir "$app/Contents/Resources/assets/fonts"
assert_dir "$app/Contents/Resources/assets/icons"
assert_file "$local_case/dist/MdowNative-1.2.3-arm64-mac-beta.zip"
assert_file "$local_case/dist/MdowNative-mac-beta.zip"
assert_contains "$local_case/log/cargo" \
  "build --release --locked --manifest-path $ROOT_DIR/apps/gpui/Cargo.toml"
assert_contains "$local_case/log/codesign" "--sign -"
assert_file "$local_case/log/verify-assets"
[[ -z "$(find "$local_case/tmp" -mindepth 1 -maxdepth 1 -print -quit)" ]] || \
  fail "local package left temporary directories behind"
case "$(<"$local_case/log/verify-assets")" in
  "$ROOT_DIR"/*) fail "asset verification resolved into repository" ;;
esac
print "PASS: local package assembles, signs ad-hoc, and validates after extraction"

wrong_asset_root_case="$test_dir/wrong-asset-root"
if run_packager \
  "$wrong_asset_root_case" \
  arm64 \
  FAKE_VERIFY_ASSET_ROOT="$ROOT_DIR/apps/gpui/assets" \
  >"$wrong_asset_root_case.output" 2>&1; then
  fail "checkout-relative asset root unexpectedly passed extracted package validation"
fi
assert_contains "$wrong_asset_root_case.output" "asset root"
print "PASS: extracted package rejects a checkout-relative asset root"

for plist_case in \
  "CFBundleExecutable|WrongExecutable|executable" \
  "CFBundleName|Wrong Name|bundle-name" \
  "CFBundleDisplayName|Wrong Display Name|display-name" \
  "CFBundleIdentifier|com.example.wrong|identifier" \
  "LSMinimumSystemVersion|13.0|minimum-system" \
  "CFBundleShortVersionString|9.9.9|short-version" \
  "CFBundleVersion|999|build-number"; do
  plist_key="${plist_case%%|*}"
  plist_remainder="${plist_case#*|}"
  plist_value="${plist_remainder%%|*}"
  plist_label="${plist_remainder##*|}"
  plist_case_dir="$test_dir/extracted-plist-$plist_label"

  if run_packager \
    "$plist_case_dir" \
    arm64 \
    FAKE_EXTRACTED_PLIST_KEY="$plist_key" \
    FAKE_EXTRACTED_PLIST_VALUE="$plist_value" \
    >"$plist_case_dir.output" 2>&1; then
    fail "extracted package with mismatched $plist_key unexpectedly passed validation"
  fi
  assert_contains "$plist_case_dir.output" "$plist_key"
done
print "PASS: extracted package revalidates complete plist identity, version, and build"

wrong_arch_case="$test_dir/wrong-arch"
if run_packager "$wrong_arch_case" x86_64 >"$wrong_arch_case.output" 2>&1; then
  fail "x86_64 binary unexpectedly packaged"
fi
[[ ! -s "$wrong_arch_case/log/codesign" ]] || fail "wrong architecture reached signing"
assert_contains "$wrong_arch_case.output" "arm64-only"
assert_not_contains "$wrong_arch_case.output" "Refusing to clean unvalidated temporary path"
print "PASS: non-arm64 binary fails before signing"

unsigned_ci_case="$test_dir/unsigned-ci"
if run_packager \
  "$unsigned_ci_case" \
  arm64 \
  CI=true \
  NATIVE_MAC_CODESIGN_IDENTITY=- \
  >"$unsigned_ci_case.output" 2>&1; then
  fail "unsigned CI package unexpectedly succeeded"
fi
[[ ! -s "$unsigned_ci_case/log/codesign" ]] || fail "unsigned CI package reached signing"
assert_contains "$unsigned_ci_case.output" "signing identity"
print "PASS: CI requires a real signing identity"

apple_development_case="$test_dir/apple-development"
if run_packager \
  "$apple_development_case" \
  arm64 \
  CI=true \
  "NATIVE_MAC_CODESIGN_IDENTITY=Apple Development: Test (TEAM123)" \
  "FAKE_CODESIGN_AUTHORITY=Apple Development: Test (TEAM123)" \
  APPLE_ID=test@example.com \
  APPLE_APP_SPECIFIC_PASSWORD=test-password \
  APPLE_TEAM_ID=TEAM123 \
  >"$apple_development_case.output" 2>&1; then
  fail "Apple Development signature unexpectedly passed CI release validation"
fi
[[ ! -e "$apple_development_case/dist/MdowNative-1.2.3-arm64-mac-beta.zip" ]] || \
  fail "Apple Development signature produced a release archive"
assert_contains "$apple_development_case.output" "Developer ID Application"
print "PASS: CI rejects Apple Development signatures"

hash_identity_case="$test_dir/hash-identity"
if run_packager \
  "$hash_identity_case" \
  arm64 \
  CI=true \
  NATIVE_MAC_CODESIGN_IDENTITY=0123456789ABCDEF0123456789ABCDEF01234567 \
  "FAKE_CODESIGN_AUTHORITY=Developer ID Application: Test (TEAM123)" \
  >"$hash_identity_case.output" 2>&1; then
  fail "Developer ID certificate hash bypassed CI notarization credentials"
fi
[[ ! -e "$hash_identity_case/dist/MdowNative-1.2.3-arm64-mac-beta.zip" ]] || \
  fail "unnotarized certificate-hash package produced a release archive"
assert_contains "$hash_identity_case.output" "notarization credentials"
print "PASS: CI certificate hash still requires complete notarization"

release_case="$test_dir/release"
run_packager \
  "$release_case" \
  arm64 \
  CI=true \
  "NATIVE_MAC_CODESIGN_IDENTITY=Developer ID Application: Test (TEAM123)" \
  APPLE_ID=test@example.com \
  APPLE_APP_SPECIFIC_PASSWORD=test-password \
  APPLE_TEAM_ID=TEAM123 \
  >"$release_case.output" 2>&1
assert_contains "$release_case/log/xcrun" "notarytool submit"
assert_contains "$release_case/log/xcrun" "--wait"
assert_contains "$release_case/log/xcrun" "stapler staple"
assert_contains "$release_case/log/xcrun" "stapler validate"
assert_contains "$release_case/log/spctl" "--type execute"
assert_file "$release_case/log/verify-assets"
[[ -z "$(find "$release_case/tmp" -mindepth 1 -maxdepth 1 -print -quit)" ]] || \
  fail "release package left temporary directories behind"
case "$(<"$release_case/log/verify-assets")" in
  "$ROOT_DIR"/*) fail "release asset verification resolved into repository" ;;
esac
print "PASS: release package is notarized, stapled, extracted, assessed, and asset-verified"

print "PASS: GPUI mac beta packaging contract"
