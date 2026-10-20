#!/usr/bin/env bash
# Builds a debug bundle and runs it in the foreground, passing files or folders through.
#   scripts/run.sh README.md docs/
set -euo pipefail

cd "$(dirname "$0")/.."
CONFIGURATION=debug bash scripts/bundle.sh >/dev/null
paths=()
for arg in "$@"; do
  if [[ -e "$arg" ]]; then paths+=("$(cd "$(dirname "$arg")" && pwd -P)/$(basename "$arg")"); else paths+=("$arg"); fi
done
exec "dist/Mdow.app/Contents/MacOS/Mdow" "${paths[@]+"${paths[@]}"}"
