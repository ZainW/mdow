#!/bin/zsh

emulate -L zsh
set -euo pipefail

readonly script_directory="${0:A:h}"
readonly repository_root="${script_directory:h}"
readonly manifest_path="$repository_root/apps/gpui/Cargo.toml"
readonly cargo_target_directory="${repository_root:A}/apps/gpui/target"
readonly bench_out="${MDOW_READER_BENCH_OUT:-$cargo_target_directory/reader-scroll-bench.json}"

export DEVELOPER_DIR="${DEVELOPER_DIR:-/Applications/Xcode.app/Contents/Developer}"
export CARGO_TARGET_DIR="$cargo_target_directory"
export MDOW_READER_BENCH_OUT="$bench_out"

mkdir -p "${bench_out:h}"

(
  cd "$repository_root"
  cargo test --manifest-path "$manifest_path" --lib reader_scroll_cost_on_a_large_document -- --nocapture
)

print -r -- "wrote $bench_out"
if [[ -f "$bench_out" ]]; then
  cat "$bench_out"
fi
