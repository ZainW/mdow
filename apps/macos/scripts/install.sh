#!/usr/bin/env bash
# Builds a release bundle and installs it in ~/Applications (or $DEST).
#   scripts/install.sh
#   DEST=/Applications scripts/install.sh
#   scripts/install.sh --uninstall
set -euo pipefail

cd "$(dirname "$0")/.."
DEST="${DEST:-$HOME/Applications}"
APP="Mdow.app"

if [[ "${1:-}" == "--uninstall" ]]; then
  rm -rf "${DEST:?}/$APP"
  echo "Removed $DEST/$APP"
  exit 0
fi

bash scripts/bundle.sh
mkdir -p "$DEST"
rm -rf "${DEST:?}/$APP"
cp -R "dist/$APP" "$DEST/"
# Let Launch Services know about its document types right away.
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$DEST/$APP" || true
echo "Installed $DEST/$APP"
