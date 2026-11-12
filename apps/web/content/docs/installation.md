---
title: Installation
description: Platform-specific installation instructions
category: Basics
order: 2
---

# Installation

Download the latest release from the [download page](/download) or [GitHub Releases](https://github.com/ZainW/mdow/releases).

## macOS

Mdow needs an Apple Silicon Mac running macOS 15 or newer. Download `Mdow-Rune-<version>.zip`
from the [download page](/download), unzip it, and move `Mdow Rune.app` to Applications. It's
signed and notarized.

It is the same app as Mdow for Linux, drawn by its own GPU renderer, with the Mac's menu bar,
Finder and web views where they belong.

If you have the earlier Mdow for Mac (`Mdow.app`, built in Swift), the two sit side by side
with their own settings. The Swift app keeps updating itself for now, but new work goes into
this one. The Electron version and Mdow Native's beta are retired; copies of either keep
working but no longer get updates.

## Linux (preview)

Mdow for Linux is the same app as on the Mac, for x64, on any desktop with Wayland or X11 and
a GPU with Vulkan or OpenGL. It's in preview, published as the
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

On [Omarchy](https://omarchy.org) it takes its colours and corners from the current theme and
follows `omarchy theme set`. Settings › Appearance › Light or Dark goes back to Mdow's own
colours.

## Windows

Mdow is no longer built for Windows. Copies of the earlier Electron app keep working but no
longer get updates.

## Updates

Mdow doesn't update itself yet. Download the new version from the [download page](/download)
when the [changelog](/changelog) shows one; on Linux, download a newer pre-release.
