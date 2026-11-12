# Mdow

Mdow on [Rune](https://github.com/anomalyco/rune), a Rust UI framework with its own GPU renderer,
text stack, layout and springs. It is one app for Linux and macOS: the same reader, chrome and
behaviour on both, with each system's own pieces where they belong (the Mac's menu bar, Finder
and web views; on Linux the desktop's file manager and editor, Ctrl for ⌘, and the Omarchy
theme). It is the app the site offers.

Away from Omarchy the look is Mdow's own: warm paper in light mode, neutral ink in dark, one clay
accent, Charter for text, Inter for the interface and Geist Mono for code.

- **macOS**: **Mdow Rune.app** (`com.zain.mdow.Rune`), with settings in
  `~/Library/Application Support/Mdow Rune/settings.json`. It installs beside the Swift app in
  `apps/macos`, which it replaced.
- **Linux**: the `mdow` binary, with a desktop entry (`com.zain.mdow`) and settings in
  `~/.config/mdow/settings.json`. CI packages it as an AppImage and a Flatpak
  (`packaging/`). It replaced the GTK app in `apps/native-linux`.

## Run, test, install

From the repo root, on either system:

```sh
pnpm run dev:rune README.md docs/   # debug build, run in the foreground
pnpm run test:rune                  # cargo test
pnpm run install:rune               # macOS: a release bundle in ~/Applications (DEST=…)
                                    # Linux: ~/.local/bin/mdow, desktop entry and icon (PREFIX=…)
```

Or in this folder: `cargo run -- README.md`, `cargo test`, and on the Mac `scripts/bundle.sh`,
which writes `dist/Mdow Rune.app`, signed ad hoc unless `SIGN_IDENTITY` is set;
`packaging/appimage.sh` builds the AppImage. Rust 1.99.1 is pinned in `rust-toolchain.toml`, as
Rune requires. Rune is pre-release, so `Cargo.toml` pins a rev of its `dev` branch.

On the Mac, run the bundle rather than the bare binary when you need Finder's "Open With", the
Dock icon or the app icon. `cargo run` is fine for everything else.

## On Linux

- **Keys**: Ctrl where the Mac has ⌘, written out in tooltips and the shortcuts list
  ("Ctrl+Shift+O"). The few chords that need both ⌘ and ⌃ on the Mac take the keys the GTK app
  used: Ctrl+B for the sidebar, Ctrl+Alt+1–3 for its panes. Ctrl+Q quits, Alt+← and Alt+→ go
  back and forward, Ctrl+Tab and Ctrl+Alt+→ change tabs, and F5 reloads. There's no menu bar:
  everything is in the header, the ⋯ menu, Quick Open (Ctrl+K) and the keys.
- **Windows**: the desktop draws the frame, so there's no gap for window buttons. Closing the
  window ends the app, and the last session's tabs come back the next time it starts.
- **Omarchy**: with Appearance on Automatic, the colours come from the current theme's
  `colors.toml` (`src/omarchy.rs`), text that would be hard to read on the theme's surfaces is
  nudged until it isn't, corners follow Hyprland's `decoration:rounding`, and `omarchy theme
  set` recolours open windows, diagrams and formulas included. Light or Dark goes back to
  Mdow's own colours. `MDOW_OMARCHY=off` turns it off; `MDOW_OMARCHY_THEME=<dir>` reads a
  theme from somewhere else. Open in Editor uses `omarchy-launch-editor` where it exists.
- **Files**: Show in Files asks the file manager to select the file (the
  `org.freedesktop.FileManager1` name), and linked files open with `xdg-open`.
- **HTML files** are read as documents (converted to markdown and drawn like any other), since
  there's no WebKit to show the page in. Scripts and styles never run.
- **Startup**: Vulkan isn't asked to load NVIDIA's driver on a machine with no NVIDIA GPU
  (`platform::skip_absent_gpu_drivers`). Where its packages are installed without one, that
  question took half a second before the first frame.
- **Not on Linux yet**: printing and PDF export (WebKit does them on the Mac), Share, Speech,
  Look Up, full screen from inside the app, and a window app ID (Rune doesn't set one, so
  docks show a generic icon for the running window). A second launch opens a second window
  rather than handing its files to the first.

### Checking changes without clicking around

Rune renders any app headlessly with the same renderer, so the window can be checked from a
shell:

```sh
cargo run -- --snap out.png --size 1200x800 --theme light README.md
cargo run -- --snap out.png --scene quick-open README.md    # also: settings, shortcuts, files,
                                                            # recents, split, companion, dark
cargo run -- --tree tree.json README.md                     # element tree with rects
cargo run -- --script steps.json --snap out.png --at 1.5 README.md
```

A script is timestamped input, for example `{"events": [{"t": 0.3, "type": "key", "name": "f",
"cmd": true}, {"t": 0.6, "type": "type", "text": "the"}]}`. Headless runs start from default
settings and never write them. `src/tests.rs` drives the window the same way from `cargo test`.

## How it's built

| Path                                                                  | What it does                                                                                                                                                                           |
| --------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/doc.rs`, `files.rs`, `highlight.rs`                              | Markdown (comrak) and HTML into blocks, folder scans, links, syntect highlighting. Shared with `apps/native-linux`.                                                                    |
| `src/reader/`                                                         | Blocks to Rune elements: one virtual row per top-level block, so long documents open and scroll as fast as short ones. Inline markdown becomes one selectable rich text per paragraph. |
| `src/media.rs`                                                        | Images (local, remote, SVG), Mermaid (mermaid-rs-renderer) and display math (latex-rust), rasterised off the main thread.                                                              |
| `src/app.rs`                                                          | A window's state: tabs, panes, history, find, and every command.                                                                                                                       |
| `src/ui.rs`, `sidebar.rs`, `quick_open.rs`, `sheets.rs`, `welcome.rs` | The header with tabs, the sidebar, Quick Open, Settings and Keyboard Shortcuts, the empty tab.                                                                                         |
| `src/menu.rs`, `windows.rs`                                           | The menu bar and Dock menu; commands go to the window in front.                                                                                                                        |
| `src/platform.rs`, `omarchy.rs`, `watch.rs`, `prefs.rs`               | What differs by system (appearance, file manager and editor hand-offs, opened documents, how shortcuts are written); the Omarchy theme; live reload; settings.                          |
| `src/companion/`                                                      | The OpenCode companion.                                                                                                                                                                |

## On the Mac: parity with the Swift app

Matches:

- **Reader**: GitHub-flavoured markdown with tables, task lists, alerts, footnotes, frontmatter
  titles, and the raw HTML READMEs use. Text selects and copies across blocks.
- **Code**: syntax colours for the grammars bat ships (more languages than the Swift highlighter),
  a language label and a copy button on hover.
- **Math**: `$$…$$` and ` ```math ` blocks are typeset natively with STIX Two Math.
- **Mermaid**: drawn natively in the palette's colours, no browser engine.
- **Windows**: tabs in the header (⌘T, ⌘W, ⌘1–⌘9, ⇧⌘[ and ⇧⌘], ⌃⇥, drag to reorder,
  middle-click to close), each with its own folder, split view (⌘\\) and back/forward history;
  more windows with ⌘N.
- **Header**: sidebar, history, Quick Open, companion and ⋯ (Show in Finder, Open in Editor,
  Copy Path, Reload, Split View) buttons, with the window buttons in it.
- **Sidebar**: Outline (follows the reading position), Files (with a filter), Recents, with
  Settings and a light/dark switch at the bottom.
- **Quick Open** (⌘K) for headings, files and commands; **Find** (⌘F, ⌘G, ⇧⌘G, ⌘E) highlights
  every match and counts them.
- **Settings** and **Keyboard Shortcuts**: theme, text and code fonts, text size, zoom, line
  spacing, text width, open in tabs, reload on change, the companion's model.
- **Behaviour**: live reload that keeps your place, drag and drop onto the window, files opened
  from Finder, the Dock or `open`, links to other documents open in place (⌘-click for a new
  tab), the last session's tabs come back, and the window frame is remembered.

Also matching, through AppKit and WebKit where Rune draws nothing of its own:

- **HTML files** open in a `WKWebView` laid over the pane, with page scripts off, darkened in dark
  mode when the page has no dark look of its own, and searched with WebKit's find.
- **Inline math** is typeset and flows with the words around it, on their baseline.
- **Print, Export as PDF and Page Setup**: the document goes to WebKit as a page in Mdow's light
  look (diagrams and formulas as SVG), so PDFs keep real text. HTML pages print as themselves.
- **Share** (the ⋯ menu), **Speech** (Edit › Speech, read by the system voice), Look Up and
  Search with Google on the selection's context menu.
- **Menus**: File › Open Recent, Edit › Find and Speech, and View › Text Width are submenus made
  in AppKit (`platform.rs`), as Rune's menus don't nest. Recent documents also reach the Dock menu.
- **Context menus** on code blocks, outline headings, files and recents; a resizable sidebar,
  split and companion panel; Settings and Keyboard Shortcuts in windows of their own.
- **⌘A then ⌘C** copies the whole document, including rows that aren't on screen.
- `MDOW_ACTIONS` and `MDOW_PERF=1` work as in the Swift app (`src/actions.rs`, `src/perf.rs`), and
  `MDOW_ACTIONS="glide=12600,15"` with `--native --trace t.jsonl --frames 900` measures scrolling.

Differs or missing:

- **Scrolling** drops a frame or two in a 900-frame glide on a quiet machine (the Swift app
  drops none), and more when the CPU is busy: Rune redraws the window each frame while it
  scrolls, where AppKit lets the compositor move what's already drawn.
- **Selection and VoiceOver** reach only the rows near the viewport (Rune builds no others);
  ⌘A and copy are handled for the whole document, but a drag across a long document can't select
  rows that have scrolled out.
- **Look Up** opens Dictionary rather than AppKit's inline popover; Services and Writing Tools
  don't apply to Rune text.
- **Updates**: no Sparkle yet, so it doesn't update itself. Releases sign and notarize it
  (`.github/workflows/release.yml`).
