A native Linux build of Mdow: Rust on GTK 4 + libadwaita, no web view or cross-platform UI layer. It looks and behaves like the desktop app (same design tokens, fonts, layout and shortcuts).

**AppImage** (any distro with glibc 2.39+: Ubuntu 24.04+, Debian 13+, Fedora 40+, Arch):
```sh
chmod +x Mdow-*-x86_64.AppImage
./Mdow-*-x86_64.AppImage README.md
```

**Flatpak** (any distro with Flatpak; pulls the GNOME runtime from Flathub on first install):
```sh
flatpak install --user Mdow-*-x86_64.flatpak
flatpak run com.zain.mdow.Linux README.md
```

Not included yet: the OpenCode companion, auto-updates, and math typesetting (TeX shows as source).
