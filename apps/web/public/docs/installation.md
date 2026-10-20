---
title: Installation
description: Platform-specific installation instructions
category: Basics
order: 2
---

# Installation

Download the latest release from the [download page](/download) or [GitHub Releases](https://github.com/ZainW/mdow/releases).

## macOS

On Mac, Mdow is a native app, built in Swift on AppKit and TextKit, with its own paper-and-ink
look. It needs an Apple Silicon Mac running macOS 15 or newer. Download `Mdow-<version>.zip`
from the [download page](/download), unzip it, and move `Mdow.app` to Applications.

It's signed and notarized, and updates itself: it checks in the background, and you can check
from **Mdow → Check for Updates…**.

If you have the earlier Electron version of Mdow, the new app replaces it: move it to
Applications over the old one. The Electron version doesn't update itself to the new app, and
Mdow Native's Mac beta is retired too. Copies of either keep working but no longer get updates.

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
