#!/usr/bin/env bash
# Downloads the pinned Pele build (Mermaid → SVG without a DOM) into Resources/vendor/.
set -euo pipefail

VERSION="1.6.0"
SHA256="b82d1bcd435302779a945e1e0f4a54ea56fb26e010ce3f2122f4216fcfc095d7"

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dest="$root/Resources/vendor/pele.global.js"

sha() { shasum -a 256 "$1" | cut -d' ' -f1; }

if [[ -f "$dest" && "$(sha "$dest")" == "$SHA256" ]]; then
  echo "pele $VERSION already present at $dest"
  exit 0
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

curl -sSfL "https://registry.npmjs.org/pele/-/pele-$VERSION.tgz" -o "$tmp/pele.tgz"
mkdir "$tmp/out"
tar -xzf "$tmp/pele.tgz" -C "$tmp/out" package/dist/pele.global.js

actual="$(sha "$tmp/out/package/dist/pele.global.js")"
if [[ "$actual" != "$SHA256" ]]; then
  echo "error: pele.global.js hash mismatch (expected $SHA256, got $actual)" >&2
  exit 1
fi

mkdir -p "$(dirname "$dest")"
mv "$tmp/out/package/dist/pele.global.js" "$dest"
echo "pele $VERSION written to $dest"
