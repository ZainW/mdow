Mdow for Linux: the same app as on the Mac, drawn by its own GPU renderer (Rune) rather than a toolkit or a web view. On Omarchy it takes its colours and corners from the current theme and follows `omarchy theme set`.

**AppImage** (any distro with glibc 2.39+: Ubuntu 24.04+, Debian 13+, Fedora 40+, Arch):
```sh
chmod +x Mdow-*-x86_64.AppImage
./Mdow-*-x86_64.AppImage README.md
```

**Flatpak** (any distro with Flatpak; pulls the GNOME runtime from Flathub on first install):
```sh
flatpak install --user Mdow-*-x86_64.flatpak
flatpak run com.zain.mdow README.md
```

It needs a GPU with Vulkan or OpenGL. Not included yet on Linux: printing and PDF export, and auto-updates.
