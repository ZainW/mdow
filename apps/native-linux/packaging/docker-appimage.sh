#!/usr/bin/env bash
# Build the AppImage locally inside an Ubuntu 24.04 container, the same base CI uses.
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd -P)"
docker run --rm -v "$ROOT_DIR:/src" -w /src \
  -e DIST_DIR=/src/dist/native-linux -e CARGO_TARGET_DIR=/src/apps/native-linux/target/ubuntu-24.04 \
  -e HOST_UID="$(id -u)" -e HOST_GID="$(id -g)" ubuntu:24.04 bash -euc '
    export DEBIAN_FRONTEND=noninteractive
    apt-get update -qq
    apt-get install -y -qq build-essential curl file pkg-config libgtk-4-dev libadwaita-1-dev \
      adwaita-icon-theme librsvg2-common >/dev/null
    curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal >/dev/null
    . "$HOME/.cargo/env"
    bash apps/native-linux/packaging/appimage.sh
    chown -R "$HOST_UID:$HOST_GID" /src/dist /src/apps/native-linux/target/ubuntu-24.04
  '
