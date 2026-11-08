# Mdow for Mac on Rune

Mdow for Mac rebuilt on [Rune](https://github.com/anomalyco/rune), a Rust UI framework with its
own GPU renderer, text stack, layout and springs. It aims for parity with the Swift app in
`apps/macos`: the same reader, chrome and behaviour, drawn by Rune instead of AppKit and TextKit.
The look matches the Swift app: warm paper in light mode, neutral ink in dark, one clay accent,
Charter for text, Inter for the interface and Geist Mono for code.

It installs beside the Swift app as **Mdow Rune.app** (`com.zain.mdow.Rune`), with its own
settings in `~/Library/Application Support/Mdow Rune/settings.json`.

## Run, test, install

From the repo root:

```sh
pnpm run dev:rune-mac README.md docs/   # debug bundle, run in the foreground
pnpm run test:rune-mac                  # cargo test
pnpm run install:rune-mac               # release bundle into ~/Applications (DEST=… to change)
```

Or in this folder: `cargo run -- README.md`, `cargo test`, and `scripts/bundle.sh`, which writes
`dist/Mdow Rune.app`, signed ad hoc unless `SIGN_IDENTITY` is set. Rust 1.99.1 is pinned in
`rust-toolchain.toml`, as Rune requires. Rune is pre-release, so `Cargo.toml` pins a rev of its
`dev` branch.

Run the bundle rather than the bare binary when you need Finder's "Open With", the Dock icon or
the app icon. `cargo run` is fine for everything else.

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
| `src/macos.rs`, `watch.rs`, `prefs.rs`                                | Appearance, Finder hand-offs and opened documents; live reload; settings.                                                                                                              |
| `src/companion/`                                                      | The OpenCode companion.                                                                                                                                                                |

## Parity with the Swift app

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
  in AppKit (`macos.rs`), as Rune's menus don't nest. Recent documents also reach the Dock menu.
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
- **Updates and CI**: no Sparkle and no release job yet.
