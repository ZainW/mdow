#!/usr/bin/env bash
set -euo pipefail

# Updates ZainW/homebrew-mdow with the latest desktop release.
# Prereqs: gh (authenticated), shasum, jq, git.

REPO="ZainW/mdow"
TAP_REPO="ZainW/homebrew-mdow"
PKG_JSON="$(dirname "$0")/../package.json"

VERSION="$(jq -r .version "$PKG_JSON")"
if [[ -z "$VERSION" || "$VERSION" == "null" ]]; then
  echo "could not read version from $PKG_JSON" >&2
  exit 1
fi

# Discover the canonical arm64 dmg from the published release rather than
# templating its filename — electron-builder's naming can drift.
ASSET_NAME="$(gh release view "v${VERSION}" --repo "$REPO" \
  --json assets --jq '.assets[].name | select(test("arm64\\.dmg$"))' \
  | head -n1)"
if [[ -z "$ASSET_NAME" ]]; then
  echo "no arm64 .dmg asset found on v${VERSION} of ${REPO}" >&2
  exit 1
fi
ASSET_URL="https://github.com/${REPO}/releases/download/v${VERSION}/${ASSET_NAME}"

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

echo "Downloading $ASSET_URL …"
curl -fsSL "$ASSET_URL" -o "$WORKDIR/$ASSET_NAME"

SHA="$(shasum -a 256 "$WORKDIR/$ASSET_NAME" | awk '{print $1}')"
echo "SHA-256: $SHA"

echo "Cloning $TAP_REPO …"
gh repo clone "$TAP_REPO" "$WORKDIR/tap"
cd "$WORKDIR/tap"

CASK_FILE="Casks/mdow.rb"
if [[ ! -f "$CASK_FILE" ]]; then
  echo "$CASK_FILE missing in tap repo — create it first" >&2
  exit 1
fi

# In-place sed: BSD/macOS sed wants an empty string after -i.
sed -i '' \
  -e "s|^  version \".*\"|  version \"${VERSION}\"|" \
  -e "s|^  sha256 \".*\"|  sha256 \"${SHA}\"|" \
  "$CASK_FILE"

if git diff --quiet -- "$CASK_FILE"; then
  echo "No changes to $CASK_FILE — already at $VERSION."
  exit 0
fi

git add "$CASK_FILE"
git -c user.name="mdow-release-bot" -c user.email="release@mdow.app" \
  commit -m "mdow ${VERSION}"
git push origin HEAD

echo "Tap updated to ${VERSION}."
