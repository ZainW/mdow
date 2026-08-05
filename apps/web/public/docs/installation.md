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

### Mdow Native (GPUI beta)

Mdow Native is a separate GPUI beta for Apple Silicon Macs running macOS 14 or newer. Download
`MdowNative-mac-beta.zip`, unzip it, and move `Mdow Native.app` to Applications. It runs alongside
the regular Mdow app.

Tagged releases are signed and notarized. Upgrades are manual for now: download the latest beta ZIP
and replace the old app. Mdow Native does not have an in-app updater yet.

## Windows

Download and run the installer (`.exe`). Mdow will be added to your Start menu and can be set as the default app for markdown files.

## Linux

Download the `.AppImage` file, make it executable, and run it:

```bash
chmod +x Mdow-*.AppImage
./Mdow-*.AppImage
```

AppImage requires no system installation — run it directly from your Downloads folder or move it anywhere on your PATH.

## Updates

Mdow checks for updates automatically in the background on macOS, Windows, and Linux.
When an update is available, use the in-app banner to download it and restart Mdow to install it.
You can also run a manual check from the app menu.
