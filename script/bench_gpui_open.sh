#!/bin/zsh
# Times opening, painting and live-reloading a large document in the release Mdow Native build.
#
#   script/bench_gpui_open.sh /tmp/big.md [report.json]
#
# The document is copied to a scratch folder (the run edits it to trigger live reloads) and the
# app runs with an isolated HOME so no session is restored. See apps/gpui/src/perf.rs.

emulate -L zsh
set -euo pipefail

readonly script_directory="${0:A:h}"
readonly repository_root="${script_directory:h}"
readonly binary="${MDOW_BIN:-$repository_root/apps/gpui/target/release/mdow-gpui}"
readonly source_document="${1:?usage: bench_gpui_open.sh <document.md> [report.json]}"
readonly report="${2:-${TMPDIR:-/tmp}/mdow-open-bench.json}"

if [[ -z "${MDOW_BIN:-}" ]]; then
  # An unbundled release binary resolves Sparkle from target/Frameworks.
  mkdir -p "$repository_root/apps/gpui/target/Frameworks"
  ln -sfn "$repository_root/apps/gpui/vendor/Sparkle/Sparkle.framework" \
    "$repository_root/apps/gpui/target/Frameworks/Sparkle.framework"
fi

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$scratch/home"
cp "$source_document" "$scratch/${source_document:t}"

HOME="$scratch/home" MDOW_PERF_LOG="$report" MDOW_PERF_RELOAD=1 \
  "$binary" "$scratch/${source_document:t}" >"${report}.log" 2>&1

print -r -- "wrote $report"
cat "$report"
