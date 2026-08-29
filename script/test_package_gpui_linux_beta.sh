#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
PACKAGER="$ROOT_DIR/script/package_gpui_linux_beta.sh"
test_dir="$(mktemp -d "${TMPDIR:-/tmp}/mdow-gpui-linux-package-test.XXXXXX")"

cleanup() {
  [[ -n "$test_dir" && -d "$test_dir" ]] && rm -rf "$test_dir"
}
trap cleanup EXIT

fail() {
  echo "FAIL: $*" >&2
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
  asset_root="$(cd "$(dirname "$0")/assets" && pwd -P)"
  [[ -d "$asset_root/fonts" ]]
  [[ -d "$asset_root/icons" ]]
fi
printf '%s\n' "$asset_root" | tee "$FAKE_TOOL_LOG/verify-assets"
BINARY
chmod +x "$CARGO_TARGET_DIR/release/mdow-gpui"
FAKE

  chmod +x "$fake_dir/cargo"
}

run_packager() {
  local case_dir="$1"
  shift

  mkdir -p "$case_dir/log" "$case_dir/target" "$case_dir/dist" "$case_dir/tmp"
  make_fakes "$case_dir/fakes"

  env \
    ARCH=x64 \
    VERSION=1.2.3 \
    DIST_DIR="$case_dir/dist" \
    CARGO_TARGET_DIR="$case_dir/target" \
    TMPDIR="$case_dir/tmp" \
    FAKE_TOOL_LOG="$case_dir/log" \
    FAKE_REPO_ROOT="$ROOT_DIR" \
    CARGO="$case_dir/fakes/cargo" \
    "$@" \
    bash "$PACKAGER"
}

[[ -f "$PACKAGER" ]] || fail "missing GPUI Linux packager: $PACKAGER"

node - "$ROOT_DIR/package.json" <<'JS'
const fs = require('node:fs')
const packageJson = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'))
const scripts = packageJson.scripts ?? {}
if (scripts['package:gpui-linux-beta'] !== 'bash script/package_gpui_linux_beta.sh') process.exit(1)
if (scripts['test:package:gpui-linux-beta'] !== 'bash script/test_package_gpui_linux_beta.sh') {
  process.exit(1)
}
JS

repo_tmp_case="$test_dir/repo-tmp"
if run_packager "$repo_tmp_case" TMPDIR=. >"$repo_tmp_case.output" 2>&1; then
  fail "repository-contained TMPDIR unexpectedly packaged"
fi
assert_contains "$repo_tmp_case.output" "Temporary directory base must be outside repository"
[[ ! -s "$repo_tmp_case/log/cargo" ]] || fail "repository-contained TMPDIR reached Cargo"
echo "PASS: repository-contained relative TMPDIR fails before building"

local_case="$test_dir/local"
run_packager "$local_case" >"$local_case.output" 2>&1

bundle="$local_case/dist/MdowNative-1.2.3-x64-linux-beta"
assert_file "$bundle/MdowNative"
assert_dir "$bundle/assets/fonts"
assert_dir "$bundle/assets/icons"
assert_file "$local_case/dist/MdowNative-1.2.3-x64-linux-beta.zip"
assert_file "$local_case/dist/MdowNative-linux-beta.zip"
assert_contains "$local_case/log/cargo" \
  "build --release --locked --manifest-path $ROOT_DIR/apps/gpui/Cargo.toml"
assert_file "$local_case/log/verify-assets"
case "$(<"$local_case/log/verify-assets")" in
  "$ROOT_DIR"/*) fail "asset verification resolved into repository" ;;
esac
echo "PASS: local Linux package assembles and validates after extraction"

wrong_asset_root_case="$test_dir/wrong-asset-root"
if run_packager \
  "$wrong_asset_root_case" \
  FAKE_VERIFY_ASSET_ROOT="$ROOT_DIR/apps/gpui/assets" \
  >"$wrong_asset_root_case.output" 2>&1; then
  fail "checkout-relative asset root unexpectedly passed package validation"
fi
assert_contains "$wrong_asset_root_case.output" "asset root"
echo "PASS: package rejects a checkout-relative asset root"

echo "PASS: GPUI Linux packager contracts"
