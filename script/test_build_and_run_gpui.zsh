#!/bin/zsh

emulate -L zsh
set -u
setopt pipe_fail
unsetopt bg_nice
zmodload zsh/datetime

readonly launcher_source="${0:A:h}/build_and_run_gpui.sh"
typeset -gi tests_run=0
typeset -gi failures=0
typeset -a cleanup_pids=()
typeset -a cleanup_dirs=()

function cleanup {
  local pid directory
  for pid in "${cleanup_pids[@]}"; do
    if kill -0 "$pid" 2>/dev/null; then
      kill -TERM "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    fi
  done
  for directory in "${cleanup_dirs[@]}"; do
    [[ -d "$directory" ]] && rm -rf -- "$directory"
  done
}

trap cleanup EXIT INT TERM HUP

function fail {
  print -u2 -r -- "FAIL: $1"
  (( failures += 1 ))
}

function pass {
  print -r -- "PASS: $1"
}

function assert_equal {
  local expected="$1"
  local actual="$2"
  local message="$3"
  if [[ "$actual" == "$expected" ]]; then
    return 0
  fi
  fail "$message (expected ${(qqq)expected}, got ${(qqq)actual})"
  return 1
}

function assert_contains_line {
  local expected="$1"
  local file="$2"
  local message="$3"
  if /usr/bin/grep -Fqx -- "$expected" "$file"; then
    return 0
  fi
  fail "$message"
  return 1
}

function wait_for_line {
  local file="$1"
  local attempts=100
  while (( attempts > 0 )); do
    [[ -s "$file" ]] && return 0
    /bin/sleep 0.02
    (( attempts -= 1 ))
  done
  return 1
}

function write_executable {
  local destination="$1"
  local body="$2"
  print -r -- "$body" > "$destination"
  chmod +x "$destination"
}

function new_fixture {
  local fixture_root
  fixture_root="$(/usr/bin/mktemp -d /private/tmp/mdow-gpui-launcher-test.XXXXXX)" || return 1
  cleanup_dirs+=("$fixture_root")

  export TEST_REPO="$fixture_root/repository with spaces"
  export TEST_FAKE_BIN="$fixture_root/fake bin"
  export TEST_DEVELOPER_DIR="$fixture_root/Fake Xcode.app/Contents/Developer"
  export TEST_TOOLCHAIN_ID="com.example.dt.toolchain.Metal.42"
  export TEST_CARGO_LOG="$fixture_root/cargo.log"
  export TEST_XCRUN_LOG="$fixture_root/xcrun.log"
  export TEST_APP_LOG="$fixture_root/app.log"

  /bin/mkdir -p \
    "$TEST_REPO/script" \
    "$TEST_REPO/apps/gpui/src" \
    "$TEST_REPO/apps/gpui/tests/fixtures" \
    "$TEST_REPO/apps/gpui/target/debug" \
    "$TEST_REPO/empty-zdot" \
    "$TEST_FAKE_BIN" \
    "$TEST_DEVELOPER_DIR/usr/bin" \
    "$TEST_DEVELOPER_DIR/Platforms/MacOSX.platform" \
    "$TEST_DEVELOPER_DIR/Toolchains/Metal Test.xctoolchain"

  /bin/cp "$launcher_source" "$TEST_REPO/script/build_and_run_gpui.sh"
  chmod +x "$TEST_REPO/script/build_and_run_gpui.sh"
  : > "$TEST_REPO/apps/gpui/Cargo.toml"
  print -r -- '# Showcase' > "$TEST_REPO/apps/gpui/tests/fixtures/showcase.md"
  : > "$TEST_DEVELOPER_DIR/usr/bin/xcodebuild"
  chmod +x "$TEST_DEVELOPER_DIR/usr/bin/xcodebuild"

  print -r -- '<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Identifier</key>
  <string>com.example.dt.toolchain.Metal.42</string>
</dict>
</plist>' > "$TEST_DEVELOPER_DIR/Toolchains/Metal Test.xctoolchain/ToolchainInfo.plist"

  write_executable "$TEST_FAKE_BIN/xcrun" '#!/bin/zsh
print -r -- "${DEVELOPER_DIR-}|${TOOLCHAINS-}|$*" >> "$TEST_XCRUN_LOG"
if [[ "${MDOW_TEST_XCRUN_FAIL-0}" == 1 ]]; then
  print -u2 -r -- "error: missing Metal Toolchain"
  exit 1
fi
if [[ "${DEVELOPER_DIR-}" == "$TEST_DEVELOPER_DIR" \
  && "${TOOLCHAINS-}" == "$TEST_TOOLCHAIN_ID" ]]; then
  if [[ "$*" == "-sdk macosx metal -v" ]]; then
    print -r -- "Apple metal version 42"
    exit 0
  fi
  if [[ "$*" == "-sdk macosx metallib -v" ]]; then
    print -r -- "Apple metallib version 42"
    exit 0
  fi
fi
print -u2 -r -- "error: missing Metal Toolchain"
exit 1'

  write_executable "$TEST_FAKE_BIN/cargo" '#!/bin/zsh
print -r -- "$PWD|${DEVELOPER_DIR-}|${TOOLCHAINS-}|${CARGO_TARGET_DIR-}|$*" >> "$TEST_CARGO_LOG"
exit "${MDOW_TEST_CARGO_STATUS-0}"'

  local fixture_source="$fixture_root/fixture.c"
  print -r -- '#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

static volatile sig_atomic_t running = 1;

static void stop(int signal_number) {
  (void)signal_number;
  running = 0;
}

int main(int argc, char **argv) {
  const char *log_path = getenv("MDOW_TEST_APP_LOG");
  if (!log_path) log_path = MDOW_TEST_DEFAULT_LOG;
  FILE *log = log_path ? fopen(log_path, "a") : NULL;
  if (log) {
    fprintf(log, "%d|%s\n", getpid(), argc > 1 ? argv[1] : "<none>");
    fclose(log);
  }
  if (getenv("MDOW_TEST_APP_STAY")) {
    signal(SIGTERM, stop);
    signal(SIGINT, stop);
    while (running) usleep(10000);
  }
  return 0;
}' > "$fixture_source"
  /usr/bin/cc -DMDOW_TEST_DEFAULT_LOG="\"$TEST_APP_LOG\"" \
    "$fixture_source" -o "$TEST_REPO/apps/gpui/target/debug/mdow-gpui" || return 1

  export TEST_PATH="$TEST_FAKE_BIN:/usr/bin:/bin:/usr/sbin:/sbin"
}

function launcher_env {
  env -u TOOLCHAINS \
    PATH="$TEST_PATH" \
    ZDOTDIR="$TEST_REPO/empty-zdot" \
    DEVELOPER_DIR="$TEST_DEVELOPER_DIR" \
    TEST_DEVELOPER_DIR="$TEST_DEVELOPER_DIR" \
    TEST_TOOLCHAIN_ID="$TEST_TOOLCHAIN_ID" \
    TEST_CARGO_LOG="$TEST_CARGO_LOG" \
    TEST_XCRUN_LOG="$TEST_XCRUN_LOG" \
    MDOW_TEST_APP_LOG="$TEST_APP_LOG" \
    "$@"
}

function test_invalid_arguments_stop_before_build {
  (( tests_run += 1 ))
  new_fixture || {
    fail "invalid-argument fixture setup"
    return
  }
  local output="$TEST_REPO/invalid.out"

  local -a invalid_commands=(
    '--unknown'
    'document.md --verify'
    'document.md -- another.md'
    '--verify --verify'
  )
  local command_line exit_status
  local all_rejected=1
  for command_line in "${invalid_commands[@]}"; do
    launcher_env "$TEST_REPO/script/build_and_run_gpui.sh" \
      ${(z)command_line} >"$output" 2>&1
    exit_status=$?
    if ! assert_equal 64 "$exit_status" \
      "invalid grammar ${(qqq)command_line} returns EX_USAGE"; then
      all_rejected=0
    fi
    if ! /usr/bin/grep -Fq 'Usage:' "$output"; then
      fail "invalid grammar ${(qqq)command_line} omitted usage"
      all_rejected=0
    fi
  done

  if (( all_rejected == 1 )) && [[ ! -e "$TEST_CARGO_LOG" ]]; then
    pass "invalid arguments stop before build"
  else
    [[ -e "$TEST_CARGO_LOG" ]] && fail "invalid arguments invoked Cargo"
  fi
}

function test_any_cwd_and_space_preservation {
  (( tests_run += 1 ))
  new_fixture || {
    fail "space-path fixture setup"
    return
  }
  local elsewhere
  local launch_path
  elsewhere="$(/usr/bin/mktemp -d /private/tmp/mdow-gpui-elsewhere.XXXXXX)"
  cleanup_dirs+=("$elsewhere")
  launch_path="$elsewhere/-document with spaces.md"
  print -r -- '# Space path' > "$launch_path"

  (
    cd "$elsewhere" || exit 1
    CARGO_TARGET_DIR="$elsewhere/hostile-target" launcher_env \
      "$TEST_REPO/script/build_and_run_gpui.sh" -- './-document with spaces.md'
  )
  local exit_status=$?
  local app_argument="$(/usr/bin/tail -n 1 "$TEST_APP_LOG" | /usr/bin/cut -d '|' -f 2-)"
  local cargo_line="$(/usr/bin/tail -n 1 "$TEST_CARGO_LOG")"

  if assert_equal 0 "$exit_status" "foreground launch exits with the application" \
    && assert_equal "$launch_path" "$app_argument" \
      "relative leading-dash path is made absolute and remains one argument" \
    && assert_equal \
      "$TEST_REPO|$TEST_DEVELOPER_DIR|$TEST_TOOLCHAIN_ID|$TEST_REPO/apps/gpui/target|build --manifest-path $TEST_REPO/apps/gpui/Cargo.toml" \
      "$cargo_line" "Cargo runs from the repository with an owned target directory" \
    && assert_contains_line \
      "$TEST_DEVELOPER_DIR|$TEST_TOOLCHAIN_ID|-sdk macosx metal -v" \
      "$TEST_XCRUN_LOG" "launcher did not validate metal" \
    && assert_contains_line \
      "$TEST_DEVELOPER_DIR|$TEST_TOOLCHAIN_ID|-sdk macosx metallib -v" \
      "$TEST_XCRUN_LOG" "launcher did not validate metallib"; then
    pass "launcher works from any cwd and preserves spaces"
  fi
}

function test_verify_defaults_to_showcase_and_reaps_child {
  (( tests_run += 1 ))
  new_fixture || {
    fail "verify fixture setup"
    return
  }
  local output="$TEST_REPO/verify.out"

  MDOW_TEST_APP_STAY=1 launcher_env \
    "$TEST_REPO/script/build_and_run_gpui.sh" --verify >"$output" 2>&1
  local exit_status=$?
  local child_line="$(/usr/bin/tail -n 1 "$TEST_APP_LOG")"
  local child_pid="${child_line%%|*}"
  local child_argument="${child_line#*|}"

  if assert_equal 0 "$exit_status" "verify exits zero" \
    && assert_contains_line 'Mdow GPUI verification passed' "$output" \
      "verify omitted the exact success line" \
    && assert_equal "$TEST_REPO/apps/gpui/tests/fixtures/showcase.md" "$child_argument" \
      "verify without a path uses the showcase fixture" \
    && ! kill -0 "$child_pid" 2>/dev/null; then
    pass "verify polls, reports success, terminates, and reaps its child"
  else
    kill -0 "$child_pid" 2>/dev/null && fail "verify left its child running"
  fi
}

function test_exact_path_prior_process_only {
  (( tests_run += 1 ))
  new_fixture || {
    fail "exact-process fixture setup"
    return
  }
  local target_binary="$TEST_REPO/apps/gpui/target/debug/mdow-gpui"
  local other_directory="$TEST_REPO/unrelated app"
  local other_binary="$other_directory/mdow-gpui"
  /bin/mkdir -p "$other_directory"
  /bin/cp "$target_binary" "$other_binary"

  MDOW_TEST_APP_STAY=1 MDOW_TEST_APP_LOG="$TEST_APP_LOG" "$target_binary" old-target &
  local target_pid=$!
  cleanup_pids+=("$target_pid")
  MDOW_TEST_APP_STAY=1 MDOW_TEST_APP_LOG="$TEST_APP_LOG" "$other_binary" unrelated &
  local other_pid=$!
  cleanup_pids+=("$other_pid")
  wait_for_line "$TEST_APP_LOG" || {
    fail "fixture processes did not start"
    return
  }
  /bin/sleep 0.05

  local output="$TEST_REPO/exact-process.out"
  local -F started_at=$EPOCHREALTIME
  MDOW_TEST_APP_STAY=1 launcher_env \
    "$TEST_REPO/script/build_and_run_gpui.sh" --verify >"$output" 2>&1
  local exit_status=$?
  local -F elapsed=$(( EPOCHREALTIME - started_at ))

  if assert_equal 0 "$exit_status" "verify after old process exits zero" \
    && ! kill -0 "$target_pid" 2>/dev/null \
    && kill -0 "$other_pid" 2>/dev/null \
    && (( elapsed < 2.0 )); then
    pass "only an earlier process with the exact executable path is terminated"
  else
    kill -0 "$target_pid" 2>/dev/null && fail "exact-path earlier process survived"
    kill -0 "$other_pid" 2>/dev/null || fail "same-name unrelated process was terminated"
    (( elapsed < 2.0 )) || fail "zombie-aware prior-process shutdown took ${elapsed}s"
  fi
}

function test_foreground_exec_preserves_pid_and_signal_target {
  (( tests_run += 1 ))
  new_fixture || {
    fail "foreground fixture setup"
    return
  }

  env -u TOOLCHAINS \
    PATH="$TEST_PATH" \
    ZDOTDIR="$TEST_REPO/empty-zdot" \
    DEVELOPER_DIR="$TEST_DEVELOPER_DIR" \
    TEST_DEVELOPER_DIR="$TEST_DEVELOPER_DIR" \
    TEST_TOOLCHAIN_ID="$TEST_TOOLCHAIN_ID" \
    TEST_CARGO_LOG="$TEST_CARGO_LOG" \
    TEST_XCRUN_LOG="$TEST_XCRUN_LOG" \
    MDOW_TEST_APP_LOG="$TEST_APP_LOG" \
    MDOW_TEST_APP_STAY=1 \
    "$TEST_REPO/script/build_and_run_gpui.sh" &
  local launcher_pid=$!
  cleanup_pids+=("$launcher_pid")
  wait_for_line "$TEST_APP_LOG" || {
    fail "foreground application did not start"
    return
  }
  local app_pid="$(/usr/bin/tail -n 1 "$TEST_APP_LOG" | /usr/bin/cut -d '|' -f 1)"
  kill -TERM "$launcher_pid" 2>/dev/null || true
  wait "$launcher_pid" 2>/dev/null || true

  if assert_equal "$launcher_pid" "$app_pid" "normal launch execs the foreground application" \
    && ! kill -0 "$app_pid" 2>/dev/null; then
    pass "foreground launch receives signals without an orphan"
  else
    kill -0 "$app_pid" 2>/dev/null && fail "foreground application remained orphaned"
  fi
}

function test_missing_metal_is_actionable_and_skips_build {
  (( tests_run += 1 ))
  new_fixture || {
    fail "missing-Metal fixture setup"
    return
  }
  local output="$TEST_REPO/missing-metal.out"

  MDOW_TEST_XCRUN_FAIL=1 launcher_env \
    "$TEST_REPO/script/build_and_run_gpui.sh" >"$output" 2>&1
  local exit_status=$?

  if (( exit_status != 0 )) \
    && [[ ! -e "$TEST_CARGO_LOG" ]] \
    && /usr/bin/grep -Fq 'Metal Toolchain' "$output" \
    && /usr/bin/grep -Fq 'xcodebuild -downloadComponent MetalToolchain' "$output"; then
    pass "missing Metal reports an actionable error before Cargo"
  else
    (( exit_status == 0 )) && fail "missing Metal unexpectedly succeeded"
    [[ -e "$TEST_CARGO_LOG" ]] && fail "missing Metal still invoked Cargo"
    /usr/bin/grep -Fq 'Metal Toolchain' "$output" || fail "missing Metal error lacked context"
    /usr/bin/grep -Fq 'xcodebuild -downloadComponent MetalToolchain' "$output" \
      || fail "missing Metal error lacked install command"
  fi
}

function test_cargo_failure_is_returned_without_launch {
  (( tests_run += 1 ))
  new_fixture || {
    fail "Cargo-failure fixture setup"
    return
  }
  local output="$TEST_REPO/cargo-failure.out"

  MDOW_TEST_CARGO_STATUS=23 launcher_env \
    "$TEST_REPO/script/build_and_run_gpui.sh" >"$output" 2>&1
  local exit_status=$?

  if assert_equal 23 "$exit_status" "Cargo failure status propagates" \
    && [[ ! -e "$TEST_APP_LOG" ]]; then
    pass "build failure never launches the application"
  else
    [[ -e "$TEST_APP_LOG" ]] && fail "application launched after build failure"
  fi
}

if [[ ! -f "$launcher_source" ]]; then
  print -u2 -r -- "FAIL: missing production launcher $launcher_source"
  exit 1
fi

test_invalid_arguments_stop_before_build
test_any_cwd_and_space_preservation
test_verify_defaults_to_showcase_and_reaps_child
test_exact_path_prior_process_only
test_foreground_exec_preserves_pid_and_signal_target
test_missing_metal_is_actionable_and_skips_build
test_cargo_failure_is_returned_without_launch

print -r -- "Ran $tests_run launcher contract tests; $failures failed"
(( failures == 0 ))
