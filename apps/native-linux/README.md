# Mdow for Linux

Mdow, built natively for the Linux desktop: Rust on GTK 4 and libadwaita. There's no web view,
JavaScript runtime or cross-platform UI layer. Documents are drawn with real GTK widgets and
Pango text, and the app follows libadwaita conventions (CSD window, `AdwTabView` tabs,
`GtkFileDialog` via portals, `GFileMonitor` live reload, D-Bus single instance).

It shares Mdow's design tokens (converted from OKLCH), fonts, layout and shortcuts.

## Requirements

- Rust (stable, 2024 edition)
- GTK ≥ 4.14 and libadwaita ≥ 1.5, with development headers
  - Arch: `pacman -S gtk4 libadwaita`
  - Fedora 40+: `dnf install gtk4-devel libadwaita-devel`
  - Debian 13 / Ubuntu 24.04+: `apt install libgtk-4-dev libadwaita-1-dev`

The fonts (Inter, Charter, Geist Mono, JetBrains Mono) are compiled in from
`assets/fonts` and registered for the app's process only.

## Run, test, install

From the repo root:

```sh
pnpm run dev:linux               # cargo run
pnpm run test:linux              # cargo test
pnpm run install:linux           # release build into ~/.local (PREFIX=… to change)
```

Or with Cargo directly in this folder: `cargo run -- path/to/file.md`, `cargo test`,
`cargo clippy`, `cargo fmt`. `scripts/install.sh --uninstall` removes an install.

Open files and folders with `mdow-linux README.md docs/`. A second launch hands its files to
the running window. To make Mdow the default markdown viewer:
`xdg-mime default com.zain.mdow.Linux.desktop text/markdown`.

## What's in it

- **Reader**: GitHub-flavored markdown (comrak) with tables, task lists, alerts, footnotes,
  frontmatter titles, and soft line breaks shown as on GitHub. The top level of a document is a
  virtualized `GtkListView`, one row per block. Long code blocks and lists are split across rows,
  so multi-megabyte files open instantly.
- **Code**: syntect with bat's grammars, coloured with the desktop's GitHub light/dark palettes.
  It runs off the main thread and shows a copy button on hover.
- **Mermaid**: laid out by mermaid-rs-renderer in Rust, themed with the Mdow palette, and
  rasterised by resvg at the display scale. No browser engine is involved.
- **HTML files**: converted to markdown (htmd) and rendered natively. Scripts and styles never
  run.
- **Chrome**: tabs with drag-to-reorder and the desktop's context menu; a side-by-side split
  view; a breadcrumb; a sidebar with Recents, Folder (with filter) and Outline (follows your
  scroll position); find in document; command palette; settings; keyboard shortcuts; the
  hold-Ctrl cheat sheet; and a zoom indicator.
- **Behaviour**: live reload that keeps your place, session restore, drag and drop of files or
  a folder anywhere in the window, and links to other documents opening in a new tab.

## Downloads and packaging

`.github/workflows/linux.yml` builds an AppImage and a Flatpak bundle:

- **Every push to `main` that touches the app** refreshes the
  [`linux-latest`](https://github.com/ZainW/mdow/releases/tag/linux-latest) pre-release.
- **A `linux-<version>` tag** (matching `version` in `Cargo.toml`) publishes a versioned
  pre-release.
- **Pull requests** build and test only.

The AppImage is built on Ubuntu 24.04 and bundles GTK and libadwaita, so it runs on glibc 2.39+
(Ubuntu 24.04+, Debian 13+, Fedora 40+, Arch). The Flatpak runs on the GNOME 51 runtime on any
distro. To build either locally with Docker, use `packaging/docker-appimage.sh` or
`packaging/docker-flatpak.sh`; both write to `dist/native-linux/`.

## Not included

These features of the other Mdow apps are not in the Linux build:

- The OpenCode companion.
- The auto-updater (on Linux, updates come from your package manager).
- KaTeX math typesetting: TeX source is shown in the code font instead.

## Layout

| File                                                               | What it does                                                          |
| ------------------------------------------------------------------ | --------------------------------------------------------------------- |
| `src/main.rs`                                                      | `AdwApplication`, single instance, file opening                       |
| `src/window.rs`                                                    | main window, tabs, split view, actions and shortcuts, session         |
| `src/reader.rs`                                                    | the virtualized reader, find, zoom indicator, outline tracking        |
| `src/render.rs`                                                    | blocks → GTK widgets and Pango markup                                 |
| `src/doc.rs`                                                       | markdown / HTML → block model                                         |
| `src/sidebar.rs`, `src/welcome.rs`, `src/dialogs.rs`               | the rest of the UI                                                    |
| `src/theme.rs`                                                     | design tokens and the generated stylesheet                            |
| `src/media.rs`, `src/highlight.rs`, `src/files.rs`, `src/prefs.rs` | images and diagrams, syntax highlighting, files and folders, settings |

Settings and the session live in `~/.config/mdow-linux/state.json`.

### Screenshots without a display

Setting `MDOW_SCREENSHOT=out.png` renders the window offscreen to a PNG once it settles, then
quits. Pair it with GTK's Broadway backend and a headless browser as the Broadway client:

```sh
gtk4-broadwayd :5 &
chromium --headless=new http://127.0.0.1:8085/ &
GDK_BACKEND=broadway BROADWAY_DISPLAY=:5 MDOW_STATE=/tmp/state.json \
  MDOW_SCREENSHOT=/tmp/shot.png cargo run -- README.md
```

Other variables used with it:

- `MDOW_SCREENSHOT_DELAY`: milliseconds to wait before capturing.
- `MDOW_SCREENSHOT_SCROLL`: a heading anchor or row number to scroll to.
- `MDOW_SCREENSHOT_FIND`: a query to search for.
- `MDOW_SCREENSHOT_ACTION`: window actions to run, e.g. `win.theme=dark;win.settings`.
- `MDOW_SCREENSHOT_CHEAT`: show the hold-Ctrl cheat sheet.
