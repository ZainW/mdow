#!/usr/bin/env bash
# Build the Flatpak bundle locally in the same container CI uses.
# Output: <repo>/dist/native-linux/Mdow-<version>-x86_64.flatpak
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd -P)"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT_DIR/apps/native-linux/Cargo.toml" | head -1)"
mkdir -p "$ROOT_DIR/dist/native-linux"
# The named volume keeps the GNOME SDK between runs.
docker run --rm --privileged -v "$ROOT_DIR:/src" -v mdow-flatpak:/root/.local/share/flatpak \
  -w /src -e VERSION="$VERSION" \
  -e HOST_UID="$(id -u)" -e HOST_GID="$(id -g)" \
  ghcr.io/flathub-infra/flatpak-github-actions:gnome-51 bash -euc '
    flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
    flatpak-builder --user --force-clean --disable-rofiles-fuse --install-deps-from=flathub \
      --state-dir=/tmp/state \
      --repo=/tmp/repo /tmp/build apps/native-linux/packaging/com.zain.mdow.Linux.yml
    flatpak build-bundle /tmp/repo "dist/native-linux/Mdow-$VERSION-x86_64.flatpak" com.zain.mdow.Linux \
      --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo
    chown -R "$HOST_UID:$HOST_GID" dist
  '
