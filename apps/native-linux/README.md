# Mdow for Linux

Mdow, built natively for the Linux desktop: Rust on GTK 4 and libadwaita. There's no web view,
JavaScript runtime or cross-platform UI layer. Documents are drawn with real GTK widgets and
Pango text.

It has the same layout and features as [Mdow for Mac](../macos): tabs in the header, a flat
sidebar, the document on a card, Quick Open and the OpenCode companion. Away from Omarchy it uses
the Mac app's warm paper and neutral ink colours, with the desktop's window buttons.

## On Omarchy

On [Omarchy](https://omarchy.org) Mdow takes its look from the system, the way first-party apps
do. Nothing needs setting up; it notices `~/.local/state/omarchy/current/theme`.

- **Theme**: colours come from the theme's `colors.toml`, hover and selection from the `[controls]`
  tokens in its `shell.toml`, and floating cards from `[menu]`, so any theme works, stock or
  third-party. `omarchy theme set` recolours open windows at once, diagrams and code included.
- **Shape**: corners follow Hyprland's `decoration:rounding`, with flat surfaces and no shadows.
  There's no title bar or window buttons: Hyprland draws the frame, and Super+W closes the window.
- **Contrast**: text that would fall below 4.5:1 on a theme's surfaces is nudged lighter or darker,
  so low-contrast accents and comments stay readable. Colours that already pass aren't touched.
- **Type**: the interface and (on a new install) code use the system font from `omarchy font set`,
  picked up the next time Mdow starts. Documents stay in Charter unless Settings says otherwise.
- **Helpers**: Open in Editor uses `omarchy default editor`, Share uses `omarchy share`, the moon
  button in the sidebar opens the theme switcher, and Open Terminal Here uses your default terminal.
- **Tiling**: Ctrl+N opens another window for Hyprland to tile. Turn off "Open documents in tabs"
  in Settings to give every document its own window.
- **Keyboard**: with no text field focused, a document takes the same motions as the terminal apps
  around it: `j`/`k`, `d`/`u`, `gg`/`G`, `/`, `n`/`N`, `H`/`L` (back, forward), `[`/`]` (headings),
  `gt`/`gT` (tabs), `o` (Quick Open), `y` (copy the code block in view), `e` (editor), `r`
  (reload), `b` (sidebar), `q` (close tab) and `?` (shortcuts).

Settings › Theme › Light or Dark switches to Mdow's own colours. `MDOW_OMARCHY=off` turns the whole
integration off, and `MDOW_OMARCHY_THEME=<dir>` previews a theme directory without applying it.

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
pnpm run perf:linux              # time startup, opening, scrolling, zoom and restyling
```

Or with Cargo directly in this folder: `cargo run -- path/to/file.md`, `cargo test`,
`cargo clippy`, `cargo fmt`. `scripts/install.sh --uninstall` removes an install.

Open files and folders with `mdow-linux README.md docs/`. A second launch hands its files to
the running window. To make Mdow the default markdown viewer:
`xdg-mime default com.zain.mdow.Linux.desktop text/markdown`.

## What's in it

- **Reading column**: prose stops at about 70 characters a line in whatever font and size you
  read in (80 or 90 with Settings › Reading width), while code, tables, diagrams and images may
  use a wider column, since they don't wrap like prose. Full Width (Ctrl+Shift+L) lifts both caps.
- **Reader**: GitHub-flavored markdown (comrak) with tables, task lists, alerts, footnotes and
  frontmatter titles. The top level of a document is a virtualized `GtkListView`, one row per
  block. Long code blocks and lists are split across rows, so multi-megabyte files open instantly.
- **Code**: syntect with bat's grammars, matched by Oniguruma and coloured from the theme. It runs
  off the main thread and shows a copy button on hover.
- **Math**: `$…$`, `$$…$$` and ` ```math ` blocks are typeset by RaTeX and rasterised by resvg in
  the text colour. Formulas that don't parse show their TeX.
- **Mermaid**: laid out by mermaid-rs-renderer in Rust, themed with the palette, and rasterised by
  resvg at the display scale. No browser engine is involved.
- **HTML files**: converted to markdown (htmd) and rendered natively. Scripts and styles never
  run.
- **Windows**:
  - Tabs in the header (Ctrl+1–9, Ctrl+Tab, drag to reorder, and a context menu), several windows
    (Ctrl+N), and a side-by-side split view (Ctrl+\\).
  - Back and forward history per tab and pane (Ctrl+[ and Ctrl+]). Links and sidebar files open
    in place; Ctrl+click a link for a new tab.
  - A sidebar with the Outline (follows your scroll position), the Files in the folder (with a
    filter) and Recents, with Settings and the theme at the bottom.
  - Quick Open (Ctrl+K) for headings, links, files and commands, find in document, keyboard
    shortcuts, the hold-Ctrl cheat sheet and a zoom indicator.
  - The ⋯ menu (F10): Open in Editor, Show in Files, Open Terminal Here, Copy Path, Share, Export
    as PDF and Print.
  - The companion (Ctrl+Alt+0, or the speech-bubble button): chat with OpenCode about the document and
    its folder. It runs a private `opencode serve` (OpenCode 2, found on PATH, the login shell's
    PATH or the usual install folders) and shows every edit as a diff to accept or decline; shell
    commands are declined automatically.
- **Behaviour**: live reload that keeps your place, session restore, drag and drop of files or a
  folder anywhere in the window, and a folder's README opening when nothing else is.

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

These differ from Mdow for Mac:

- Selection stays within one block, because each block is its own widget. Mac selects across the
  whole document.
- HTML files are converted and rendered natively rather than shown in a web view, so their own
  styling isn't kept. WebKitGTK would be a large dependency for this.
- Find doesn't search paragraphs that contain inline formulas, or typeset formulas.
- Inline formulas sit slightly high, as GTK text views align images to the baseline, and they print
  as TeX. Formulas on their own line print typeset.
- There's no auto-updater: on Linux, updates come from your package manager.
- Each window has one folder, rather than one per tab.
- Services, Speech and Look Up are macOS features.

## Layout

| File                                                               | What it does                                                          |
| ------------------------------------------------------------------ | --------------------------------------------------------------------- |
| `src/main.rs`                                                      | `AdwApplication`, single instance, file opening                       |
| `src/window.rs`                                                    | windows, tabs, split view, history, actions and shortcuts, session    |
| `src/reader.rs`                                                    | the virtualized reader, find, zoom indicator, outline tracking        |
| `src/render.rs`                                                    | blocks → GTK widgets and Pango markup                                 |
| `src/doc.rs`                                                       | markdown / HTML → block model                                         |
| `src/sidebar.rs`, `src/welcome.rs`, `src/dialogs.rs`               | the rest of the UI                                                    |
| `src/theme.rs`                                                     | palettes, the generated stylesheet and its Omarchy layer              |
| `src/omarchy.rs`                                                   | the Omarchy theme, Hyprland's rounding, watching and helpers          |
| `src/print.rs`                                                     | printing and Export as PDF                                            |
| `src/companion/`                                                   | OpenCode client, process, conversation state and the panel            |
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

### Performance

`MDOW_PERF=1` logs how long things take as you use the app, as the Mac apps' variable of the
same name does: each step of starting up, and for every document the time from asking for it to
the frame that shows it, with how much of that was the main thread's.

`scripts/perf.sh` (`pnpm run perf:linux`) runs the lot and prints medians: startup, then
opening, scrolling, jumping to the end, find, zoom and a theme change over four documents (this
README, a long plan full of code, one with diagrams and formulas, and a few megabytes of
everything), then four tabs. It runs without a display, on Broadway as above; `--display` runs on
your desktop instead, which is the way to judge frame pacing. `--against other-binary` runs
another build beside it, to see what a change did.

A few things it measures that are worth knowing when changing the reader:

- `GtkListView` keeps up to 200 rows bound around the view. Only the ones in or near the view are
  built at once; the rest are blanks of about the right height, built a few a frame afterwards or
  as they come near. `MDOW_EAGER_ROWS=1` builds them all at once, to compare.
- A wrapping label is given a minimum width in characters. Without one GTK finds it by laying the
  text out one character a line, which costs more than the rest of measuring a document.
- Restyling only rebuilds rows when something they show has changed.
