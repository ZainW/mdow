#!/usr/bin/env bash
# Builds a debug build and runs it in the foreground with the given files:
#   scripts/run.sh README.md docs/
# On macOS it runs from a bundle, so the Dock icon and "Open With" work.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CALLER="$PWD"
cd "$ROOT"
args=()
for a in "$@"; do
  if [[ "$a" == /* || "$a" == -* ]]; then args+=("$a"); else args+=("$CALLER/$a"); fi
done
if [[ "$(uname -s)" == "Darwin" ]]; then
  CONFIGURATION=debug bash scripts/bundle.sh >/dev/null
  exec "$ROOT/dist/Mdow Rune.app/Contents/MacOS/mdow" "${args[@]}"
fi
cargo build
exec "${CARGO_TARGET_DIR:-$ROOT/target}/debug/mdow" "${args[@]}"
