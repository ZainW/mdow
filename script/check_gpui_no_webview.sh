#!/usr/bin/env zsh
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
hits="$(rg -n --glob '!target/**' -i 'webview|wkwebview|nswebview' "$root/apps/gpui" || true)"

if [[ -n "$hits" ]]; then
  print -r -- "GPUI webview gate failed:"
  print -r -- "$hits"
  exit 1
fi

print -r -- "GPUI webview gate passed"
