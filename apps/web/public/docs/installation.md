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
Applications over the old one. The Electron version and Mdow Native's beta are retired. Copies
of either keep working but no longer get updates.

## Linux (preview)

Mdow for Linux is a native GTK 4 and libadwaita app for x64. It's in preview, published as the
[`linux-latest` pre-release](https://github.com/ZainW/mdow/releases/tag/linux-latest) on GitHub
as an AppImage and a Flatpak bundle.

To run the AppImage, make it executable:

```bash
chmod +x Mdow-*-x86_64.AppImage
./Mdow-*-x86_64.AppImage
```

Or install the Flatpak bundle:

```bash
flatpak install --user Mdow-*-x86_64.flatpak
```

## Windows

Mdow is no longer built for Windows. Copies of the earlier Electron app keep working but no
longer get updates.

## Updates

The Mac app updates itself in the background. The Linux preview doesn't: download a newer
pre-release to update it.
