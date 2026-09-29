---
title: Installation
description: Platform-specific installation instructions
category: Basics
order: 2
---

# Installation

Download the latest release from the [download page](/download) or [GitHub Releases](https://github.com/ZainW/mdow/releases).

## macOS

**Recommended:** Download the `.dmg` file, open it, and drag Mdow to your Applications folder.

Alternatively, download the `.zip` for a portable version — unzip and run Mdow from anywhere.

### Mdow Native (beta)

Mdow Native is a separate beta that renders with GPUI through [gpuix](https://gpuix.dev). On Apple
Silicon Macs running macOS 14 or newer, download `MdowNative-mac-beta.zip`, unzip it, and move
`Mdow Native.app` to Applications. It runs alongside the regular Mdow app.

Tagged macOS releases are signed and notarized. Mdow Native checks for updates in the background
and offers them in a banner; you can also check from **Mdow → Check for Updates…**.

## Windows

Download and run the installer (`.exe`). Mdow will be added to your Start menu and can be set as the default app for markdown files.

## Linux

Download the `.AppImage` file, make it executable, and run it:

```bash
chmod +x Mdow-*.AppImage
./Mdow-*.AppImage
```

AppImage requires no system installation — run it directly from your Downloads folder or move it anywhere on your PATH.

### Mdow Native (beta)

Linux x64 builds ship as `MdowNative-linux-beta.AppImage`. Make it executable and run it:

```bash
chmod +x MdowNative-linux-beta.AppImage
./MdowNative-linux-beta.AppImage
```

It is a separate beta from the Electron AppImage and updates itself when run as an AppImage.

## Updates

Mdow checks for updates automatically in the background on macOS, Windows, and Linux.
When an update is available, use the in-app banner to download it and restart Mdow to install it.
You can also run a manual check from the app menu.
