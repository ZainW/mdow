#!/bin/zsh

emulate -L zsh
set -euo pipefail
setopt pipe_fail
unsetopt bg_nice

readonly script_directory="${0:A:h}"
readonly repository_root="${script_directory:h}"
readonly manifest_path="$repository_root/apps/gpui/Cargo.toml"
readonly cargo_target_directory="${repository_root:A}/apps/gpui/target"
readonly binary_path="$cargo_target_directory/debug/mdow-gpui"
readonly app_bundle="$cargo_target_directory/debug/Mdow Native.app"
readonly bundled_executable="$app_bundle/Contents/MacOS/MdowNative"
readonly showcase_path="$repository_root/apps/gpui/tests/fixtures/showcase.md"

if [[ -f "$script_directory/native_mac_bundle.sh" ]]; then
  source "$script_directory/native_mac_bundle.sh"
else
  copy_native_mac_resources() {
    mkdir -p "$2/Resources"
  }
  write_native_mac_info_plist() {
    cat >"$1" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key>
  <string>$2</string>
  <key>CFBundleIdentifier</key>
  <string>$4</string>
  <key>CFBundleName</key>
  <string>$3</string>
  <key>CFBundleDisplayName</key>
  <string>$3</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
</dict>
</plist>
PLIST
  }
fi

typeset -g selected_developer_directory=''
typeset -g selected_metal_toolchains=''
typeset -g metal_validation_error=''
typeset -g verify_pid=''

function usage {
  print -u2 -r -- "Usage: ${0:t} [--verify] [path]"
}

function die {
  print -u2 -r -- "Mdow GPUI: $1"
  exit "${2:-1}"
}

function validate_metal {
  local developer_directory="$1"
  local toolchains="${2-}"
  local xcrun_path="$3"

  local tool output
  for tool in metal metallib; do
    if [[ -n "$toolchains" ]]; then
      output="$(env DEVELOPER_DIR="$developer_directory" TOOLCHAINS="$toolchains" \
        "$xcrun_path" -sdk macosx "$tool" -v 2>&1)" || {
        metal_validation_error="$output"
        return 1
      }
    else
      output="$(env -u TOOLCHAINS DEVELOPER_DIR="$developer_directory" \
        "$xcrun_path" -sdk macosx "$tool" -v 2>&1)" || {
        metal_validation_error="$output"
        return 1
      }
    fi
  done
  metal_validation_error=''
}

function toolchain_identifier {
  local plist="$1"
  local identifier=''

  if [[ -x /usr/libexec/PlistBuddy ]]; then
    identifier="$(/usr/libexec/PlistBuddy -c 'Print :Identifier' "$plist" 2>/dev/null || true)"
    if [[ -z "$identifier" ]]; then
      identifier="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$plist" 2>/dev/null || true)"
    fi
  fi
  if [[ -z "$identifier" ]] && command -v plutil >/dev/null 2>&1; then
    identifier="$(plutil -extract Identifier raw -o - "$plist" 2>/dev/null || true)"
  fi
  print -r -- "$identifier"
}

function discover_xcode_and_metal {
  local xcrun_path
  xcrun_path="$(command -v xcrun 2>/dev/null)" \
    || die 'xcrun is unavailable. Install the Xcode command-line tools and full Xcode.'

  local -a developer_candidates=()
  [[ -n "${DEVELOPER_DIR-}" ]] && developer_candidates+=("$DEVELOPER_DIR")
  local selected_directory
  selected_directory="$(xcode-select -p 2>/dev/null || true)"
  [[ -n "$selected_directory" ]] && developer_candidates+=("$selected_directory")
  developer_candidates+=(
    /Applications/Xcode.app/Contents/Developer
    /Applications/Xcode*.app/Contents/Developer(N)
  )
  if [[ -n "${HOME-}" ]]; then
    developer_candidates+=(
      "$HOME"/Applications/Xcode.app/Contents/Developer(N)
      "$HOME"/Applications/Xcode*.app/Contents/Developer(N)
    )
  fi

  local found_full_xcode=0
  local developer_directory canonical_developer last_full_xcode=''
  local -A seen_developers=()
  for developer_directory in "${developer_candidates[@]}"; do
    case "$developer_directory" in
      *.app) developer_directory="$developer_directory/Contents/Developer" ;;
      */Contents) developer_directory="$developer_directory/Developer" ;;
    esac
    [[ -d "$developer_directory" ]] || continue
    canonical_developer="${developer_directory:A}"
    [[ -z "${seen_developers[$canonical_developer]-}" ]] || continue
    seen_developers[$canonical_developer]=1
    [[ -x "$canonical_developer/usr/bin/xcodebuild" ]] || continue
    [[ -d "$canonical_developer/Platforms/MacOSX.platform" ]] || continue
    found_full_xcode=1
    last_full_xcode="$canonical_developer"

    if [[ -n "${TOOLCHAINS-}" ]] \
      && validate_metal "$canonical_developer" "$TOOLCHAINS" "$xcrun_path"; then
      selected_developer_directory="$canonical_developer"
      selected_metal_toolchains="$TOOLCHAINS"
      return 0
    fi

    if validate_metal "$canonical_developer" '' "$xcrun_path"; then
      selected_developer_directory="$canonical_developer"
      selected_metal_toolchains=''
      return 0
    fi

    local -a toolchain_plists=(
      "$canonical_developer"/Toolchains/*.xctoolchain/ToolchainInfo.plist(N)
      "$canonical_developer"/Toolchains/*.xctoolchain/Info.plist(N)
      /Library/Developer/Toolchains/*.xctoolchain/ToolchainInfo.plist(N)
      /Library/Developer/Toolchains/*.xctoolchain/Info.plist(N)
      /var/run/com.apple.security.cryptexd/mnt/com.apple.MobileAsset.MetalToolchain-*/Metal.xctoolchain/ToolchainInfo.plist(N)
      /var/run/com.apple.security.cryptexd/mnt/com.apple.MobileAsset.MetalToolchain-*/Metal.xctoolchain/Info.plist(N)
    )
    if [[ -n "${HOME-}" ]]; then
      toolchain_plists+=(
        "$HOME"/Library/Developer/Toolchains/*.xctoolchain/ToolchainInfo.plist(N)
        "$HOME"/Library/Developer/Toolchains/*.xctoolchain/Info.plist(N)
      )
    fi

    local plist identifier
    local -A seen_identifiers=()
    for plist in "${toolchain_plists[@]}"; do
      identifier="$(toolchain_identifier "$plist")"
      [[ -n "$identifier" ]] || continue
      [[ -z "${seen_identifiers[$identifier]-}" ]] || continue
      seen_identifiers[$identifier]=1
      if validate_metal "$canonical_developer" "$identifier" "$xcrun_path"; then
        selected_developer_directory="$canonical_developer"
        selected_metal_toolchains="$identifier"
        return 0
      fi
    done
  done

  if (( found_full_xcode == 0 )); then
    die 'full Xcode was not found. Install Xcode, then set DEVELOPER_DIR to its Contents/Developer directory.'
  fi

  print -u2 -r -- 'Mdow GPUI: a working Metal Toolchain was not found for full Xcode. Install it with:'
  print -u2 -r -- "  DEVELOPER_DIR=\"$last_full_xcode\" xcodebuild -downloadComponent MetalToolchain"
  if [[ -n "$metal_validation_error" ]]; then
    print -u2 -r -- "Last xcrun diagnostic: ${metal_validation_error//$'\n'/' '}"
  fi
  exit 1
}

function executable_path_for_pid {
  local pid="$1"
  local executable
  executable="$(lsof -a -p "$pid" -d txt -Fn 2>/dev/null \
    | awk '/^n/ { sub(/^n/, ""); print; exit }')"
  executable="${executable% (deleted)}"
  [[ -n "$executable" ]] || return 1
  print -r -- "${executable:A}"
}

function process_matches_path {
  local pid="$1"
  local expected="$2"
  [[ "$pid" == <-> ]] || return 1
  kill -0 "$pid" 2>/dev/null || return 1
  local executable
  executable="$(executable_path_for_pid "$pid")" || return 1
  [[ "$executable" == "${expected:A}" ]]
}

function process_matches_binary {
  process_matches_path "$1" "$bundled_executable" \
    || process_matches_path "$1" "$binary_path"
}

function terminate_exact_process {
  local pid="$1"
  process_matches_binary "$pid" || return 0
  kill -TERM "$pid" 2>/dev/null \
    || die "could not terminate the earlier Mdow GPUI process $pid."

  local attempt
  for attempt in {1..50}; do
    process_matches_binary "$pid" || return 0
    /bin/sleep 0.02
  done

  if process_matches_binary "$pid"; then
    kill -KILL "$pid" 2>/dev/null \
      || die "could not stop the earlier Mdow GPUI process $pid."
  fi
  for attempt in {1..50}; do
    process_matches_binary "$pid" || return 0
    /bin/sleep 0.02
  done
  die "the earlier Mdow GPUI process $pid remained active after SIGKILL."
}

function terminate_earlier_binary_processes {
  local -a candidate_pids=()
  local discovered=''
  discovered="$(lsof -a -d txt -t -- "$bundled_executable" 2>/dev/null || true)"
  [[ -n "$discovered" ]] && candidate_pids+=("${(@f)discovered}")

  discovered="$(lsof -a -d txt -t -- "$binary_path" 2>/dev/null || true)"
  [[ -n "$discovered" ]] && candidate_pids+=("${(@f)discovered}")

  discovered="$(pgrep -x "${binary_path:t}" 2>/dev/null || true)"
  [[ -n "$discovered" ]] && candidate_pids+=("${(@f)discovered}")

  local pid
  local -A seen_pids=()
  for pid in "${candidate_pids[@]}"; do
    [[ "$pid" == <-> ]] || continue
    [[ -z "${seen_pids[$pid]-}" ]] || continue
    seen_pids[$pid]=1
    terminate_exact_process "$pid"
  done
}

function cleanup_verify_child {
  local pid="$verify_pid"
  [[ -n "$pid" ]] || return 0

  if process_matches_binary "$pid"; then
    kill -TERM "$pid" 2>/dev/null || true
    local attempt
    for attempt in {1..50}; do
      process_matches_binary "$pid" || break
      /bin/sleep 0.02
    done
    if process_matches_binary "$pid"; then
      kill -KILL "$pid" 2>/dev/null || true
    fi
  fi
  wait "$pid" 2>/dev/null || true
  verify_pid=''
}

function interrupt_verify {
  local status="$1"
  trap - INT TERM HUP
  cleanup_verify_child
  exit "$status"
}

typeset -i verify=0
typeset -i path_after_double_dash=0
typeset launch_path=''
if (( $# == 1 )) && [[ "$1" == -h || "$1" == --help ]]; then
  usage
  exit 0
fi
if (( $# > 0 )) && [[ "$1" == --verify ]]; then
  verify=1
  shift
fi
if (( $# > 0 )) && [[ "$1" == -- ]]; then
  path_after_double_dash=1
  shift
fi
if (( $# > 1 )) \
  || { (( $# == 1 && path_after_double_dash == 0 )) && [[ "$1" == -* ]]; }; then
  usage
  exit 64
fi
if (( $# == 1 )); then
  launch_path="${1:A}"
fi

command -v cargo >/dev/null 2>&1 \
  || die 'cargo is unavailable. Install Rust 1.93 or newer before building Mdow GPUI.'
command -v lsof >/dev/null 2>&1 \
  || die 'lsof is unavailable; exact executable-path process matching cannot be performed.'
command -v pgrep >/dev/null 2>&1 \
  || die 'pgrep is unavailable; exact executable-path process matching cannot be performed.'

discover_xcode_and_metal
export DEVELOPER_DIR="$selected_developer_directory"
if [[ -n "$selected_metal_toolchains" ]]; then
  export TOOLCHAINS="$selected_metal_toolchains"
else
  unset TOOLCHAINS
fi

(
  cd "$repository_root"
  CARGO_TARGET_DIR="$cargo_target_directory" cargo build --manifest-path "$manifest_path"
)
[[ -x "$binary_path" ]] \
  || die "Cargo completed without producing the expected executable at $binary_path."

rm -rf "$app_bundle"
mkdir -p "$app_bundle/Contents/MacOS"
if [[ -f "$repository_root/apps/desktop/resources/icon.icns" ]]; then
  copy_native_mac_resources "$repository_root" "$app_bundle/Contents" "MdowNative"
else
  mkdir -p "$app_bundle/Contents/Resources"
fi
if [[ -d "$repository_root/apps/gpui/assets" ]]; then
  mkdir -p "$app_bundle/Contents/Resources"
  cp -R "$repository_root/apps/gpui/assets" "$app_bundle/Contents/Resources/assets"
fi
cp "$binary_path" "$bundled_executable"
chmod +x "$bundled_executable"
write_native_mac_info_plist \
  "$app_bundle/Contents/Info.plist" \
  "MdowNative" \
  "Mdow Native" \
  "com.zain.mdow.gpui" \
  "14.0"
[[ -x "$bundled_executable" ]] \
  || die "Failed to wrap the debug executable at $bundled_executable."

terminate_earlier_binary_processes

if (( verify == 0 )); then
  if [[ -n "$launch_path" ]]; then
    exec "$bundled_executable" "$launch_path"
  else
    exec "$bundled_executable"
  fi
fi

[[ -n "$launch_path" ]] || launch_path="$showcase_path"
trap 'interrupt_verify 130' INT
trap 'interrupt_verify 143' TERM
trap 'interrupt_verify 129' HUP

"$bundled_executable" "$launch_path" &
verify_pid=$!
trap cleanup_verify_child EXIT

zmodload zsh/datetime
typeset -i exact_observations=0
typeset -F 6 verification_deadline=$(( EPOCHREALTIME + 10.0 ))
while (( EPOCHREALTIME < verification_deadline )); do
  if ! kill -0 "$verify_pid" 2>/dev/null; then
    verify_pid=''
    die "the Mdow GPUI process exited before verification completed."
  fi
  if process_matches_binary "$verify_pid"; then
    (( exact_observations += 1 ))
    if (( exact_observations >= 5 )); then
      cleanup_verify_child
      trap - EXIT INT TERM HUP
      print -r -- 'Mdow GPUI verification passed'
      exit 0
    fi
  else
    exact_observations=0
  fi
  /bin/sleep 0.1
done

cleanup_verify_child
trap - EXIT INT TERM HUP
die 'the Mdow GPUI process did not become verifiably active within 10 seconds.'
