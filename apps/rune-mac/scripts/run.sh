#!/usr/bin/env bash
# Builds a debug bundle and runs it in the foreground with the given files:
#   scripts/run.sh README.md docs/
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CALLER="$PWD"
cd "$ROOT"
CONFIGURATION=debug bash scripts/bundle.sh >/dev/null
args=()
for a in "$@"; do
  if [[ "$a" == /* || "$a" == -* ]]; then args+=("$a"); else args+=("$CALLER/$a"); fi
done
exec "$ROOT/dist/Mdow Rune.app/Contents/MacOS/mdow" "${args[@]}"
