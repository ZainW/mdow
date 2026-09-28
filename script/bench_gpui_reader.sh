#!/bin/zsh
# Reader benchmarks for Mdow Native.
#
#   script/bench_gpui_reader.sh                      scroll cost of a 1,200-block document
#   script/bench_gpui_reader.sh /tmp/big.md          ...plus the large-document pipeline bench
#                                                    (open, first paint, scroll with the outline
#                                                    visible, live reload) in release mode
#
# For real-app timings (window, preview, full parse, main-thread stalls) see
# script/bench_gpui_open.sh.

emulate -L zsh
set -euo pipefail

readonly script_directory="${0:A:h}"
readonly repository_root="${script_directory:h}"
readonly manifest_path="$repository_root/apps/gpui/Cargo.toml"
readonly cargo_target_directory="${repository_root:A}/apps/gpui/target"
readonly bench_out="${MDOW_READER_BENCH_OUT:-$cargo_target_directory/reader-scroll-bench.json}"
readonly pipeline_doc="${1:-${MDOW_PIPELINE_BENCH_DOC:-}}"
readonly pipeline_out="${MDOW_PIPELINE_BENCH_OUT:-$cargo_target_directory/reader-pipeline-bench.json}"

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

if [[ -n "$pipeline_doc" ]]; then
  # Release test binaries resolve Sparkle next to the target profile, like the app bundle.
  mkdir -p "$cargo_target_directory/release/Frameworks"
  ln -sfn "$repository_root/apps/gpui/vendor/Sparkle/Sparkle.framework" \
    "$cargo_target_directory/release/Frameworks/Sparkle.framework"
  (
    cd "$repository_root"
    MDOW_PIPELINE_BENCH_DOC="${pipeline_doc:A}" MDOW_PIPELINE_BENCH_OUT="$pipeline_out" \
      cargo test --release --manifest-path "$manifest_path" --lib \
      large_document_pipeline_bench -- --ignored --nocapture
  )
  print -r -- "wrote $pipeline_out"
  cat "$pipeline_out"
fi
