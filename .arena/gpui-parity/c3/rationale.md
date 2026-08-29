## Problem

GPUI 0.2.2 has no Input widget. Official `examples/input.rs` is a one-line `EntityInputHandler`. The Electron reader loop still needs find, a command palette, settings, three sidebar modes, session restore, and HTML documents without a webview. The 2026-07-31 spec refused `gpui-component` so Mdow metrics would stay authoritative. That refusal is still the right instinct for Markdown painting. It is the wrong instinct for chrome we have not written. `gpui-component` 0.5.1 on crates.io already depends on `gpui = "^0.2.2"`. We can take Input, List, Dialog, and Theme from that crate, project Electron colors onto its 108-field `ThemeColor`, and leave `pulldown-cmark` → `DocumentBlock` → `ui/reader.rs` alone.

## Usage (caller's view)

`main` calls `boot::install(cx)` as the first line of `Application::run`, then opens a window whose entity is still `MdowApp`. `boot::install` runs `gpui_component::init` and writes Mdow light tokens onto the Theme global before the first frame. Opening a path goes through `open::classify`. Markdown uses today's load/parse/prepare. HTML uses `html::prepare` plus `TextView::html`. Find, palette, settings, and shortcuts go through `Overlay::show`. The shell never stores four independent "open" booleans. Sidebar modes render `List::new` on a private delegate. The reader keeps taking `MdowTokens`, not `cx.theme()`, so the existing selector tests do not construct a Theme global.

See `usage.md` for the boot snippet and the three call sites. `sketch.rs` is derived from those, not the other way around.

## Shape

Data first.

`MdowTokens` is the named Electron palette plus compact type sizes the tests already pin (sidebar 244, reader 768, Inter 15.5 / 1.65, warm light HSLA, neutral dark HSLA). `Preferences` holds `ThemePreference`, `ReadingWidth`, `InterfaceScale`, zoom, fonts, and sidebar mode. `ColorScheme` is derived from preference plus `WindowAppearance`. Tokens are derived from that scheme. Nothing stores light and dark copies that have to be kept in sync.

`ThemeBridge` is the only writer of `gpui_component::theme::Theme`. It copies the 13 Mdow colors onto the matching `ThemeColor` fields, then fills hover, active, sidebar, tab, list, overlay, and leftover chart slots from those same HSLAs. The crate default palette is treated as contaminated input. Fonts and radius on the global become Inter Variable, Geist Mono, and 8 px, scaled by `InterfaceScale`. Chrome widgets read `cx.theme()`. The DocumentBlock reader still receives `MdowTokens`.

`OpenDocument` is `Markdown(PreparedDocument)` or `Html(PreparedHtml)`. A tab cannot be both. `classify` is the boundary. Markdown never enters `TextView`. HTML never enters `parse_document`.

`Overlay` is one optional `OverlayKind`. `show` dismisses the current kind, then opens the new one. Find is an in-tree Input bar. Palette, settings, and shortcuts are `window.open_dialog` on the Root layer that `MdowApp::render` appends. Running `show(Find)` twice focuses the existing input.

`Chrome` owns the `Entity<InputState>` and `Entity<ListState<_>>` values. `FolderDelegate`, `RecentsDelegate`, `OutlineDelegate`, and `PaletteDelegate` stay private. Callers set a sidebar mode or an overlay kind. They do not implement `ListDelegate`.

`Session` reads and writes the electron-store key names (`recents`, `lastFolder`, `sessionTabs`, `sessionActiveTabPath`, theme, fonts, scales, width). Companion and split-view keys are ignored.

Version pin. `gpui-component = "=0.5.1"` from crates.io sits on `gpui ^0.2.2`. Mdow keeps `gpui = "=0.2.2"`. That is the adoption. longbridge HEAD has already moved to Zed git + `gpui_platform`. Taking HEAD is a GPUI bump. Bootstrap becomes `gpui_platform::application()`, the lockfile replaces the GPU stack, and every `VisualTestContext` test has to be re-proven. We do not do that. A later crates.io 0.6 that drops 0.2.2 gets the same review.

`notify = "=8.2.0"` stays on `FileWatcher`. The crate pulls `notify ^7`. Two copies in `Cargo.lock`. Features `webview` and `tree-sitter-languages` stay off.

The public API is small on purpose. `boot`, `open::classify`, `Overlay::show`, `theme_bridge::sync`, `SidebarLists::set_mode`, `html_reader::render`. Behind that sit 108 color fields, Root dialog layers, List virtualization, Input IME, and HTML rewrite. Callers do not learn those. That is the depth bet. A richer widget API leaked into `MdowApp` would make every chrome change a coordinated edit.

The measured tab rail stays custom GPUI. `TabBar` from the crate is not in the assigned widget set, and the 36 / 28 / 4 tests already lock that strip.

## Synthesis decision

Arena fills this after comparing candidates. This package is the crates.io 0.5.1 adoption on pinned gpui 0.2.2.

## Tradeoffs accepted

- We accept a second `notify 7.x` in the lockfile, plus tree-sitter, html5ever, ropey 2.0.0-beta.1, rust-i18n, and lsp-types on the compile, in exchange for Input, List, Dialog, and Theme we will not hand-roll on 0.2.2.
- We accept `TextView::html` as Safari-reader HTML, no CSS, no script, no `on_link` in 0.5.1, in exchange for opening `.html` / `.htm` without WKWebView.
- We accept rewriting the focus-count chrome tests (`focus_next` 4 / 6 / 7) because List is one keyboard widget, not a nest of per-row tab stops, in exchange for keeping reader debug selectors and the DocumentBlock tests untouched.
- We accept a 108-field color projection that must be revisited when the crate adds tokens, in exchange for one authoritative Electron palette.
- We accept `boot::install_for_test` on every `#[gpui::test]` that constructs `MdowApp`, because chrome will call `cx.theme()`. Unit tests in `document`, `syntax`, `tabs`, `workspace`, and `theme` stay crate-free.
- We accept List's equal-height rule. Sidebar rows already share one compact slot. Indent is padding.
- We accept leaving the measured tab bar custom. Replacing it with `TabBar` would trade a known 36 / 28 rail for a widget that does not know those numbers.
- We accept not taking git HEAD, even when its docs look newer, so the GPUI pin and the 132 visual tests remain meaningful.

## Alternatives considered

Hand-rolled `EntityInputHandler` plus custom overlays. Callers would learn IME, selection, and focus themselves. The public API would grow by the same amount as the implementation. Lost because find and the palette are the reason to adopt the crate, and a one-line official example is not a find field.

`TextView::markdown` for the reader. One widget, short `MdowApp`. Exposes GFM-by-gpui-component to every test and drops the 132 selector and layout contracts. Lost because the assigned constraint is to keep DocumentBlock.

Git HEAD + `gpui_platform`. Newer widgets, current longbridge examples. Exposes Zed-tip GPUI to the whole crate. Lost because that is a version bump, not an adoption of 0.5.1 on 0.2.2.

Two palettes, Mdow tokens for the reader and stock `ThemeColor` for chrome. Smaller bridge. Exposes Longbridge teal on Input and List next to warm Mdow chrome. Lost because the Electron palette is the product.

This was not the only viable shape. The only shape that both adopts the crate and keeps the reader tests is "widgets for chrome, DocumentBlock for Markdown, tokens projected into Theme."

## Open questions and risks

Does `TextView` 0.5.1 load local images from rewritten absolute paths, or only http(s) and data URIs?

If a user clicks a local `.md` link inside an HTML document, what should happen? The 0.5.1 API has no link callback. Leave it inert, or parse `<a>` ourselves and overlay hit targets?

Does `gpui_component::init` require rust-i18n locale files we do not ship? What breaks in tests if they are missing?

When `Theme::global_mut` is written during `render` via `sync`, do Input and List pick up the new colors on that frame, or the next?

Comfortable reading width is 896 in the sketch. What number does Electron's CSS actually use?

Are crate `Button` default paddings still inside the 28 to 36 px chrome band after we set `font_size` to 13, or do dialogs need `.small()` plus a height cap?

## Next implementation step

Add `gpui-component = "=0.5.1"` next to `gpui = "=0.2.2"`, write `boot::install` and `theme_bridge::project_colors`, and assert the Theme global's `background`, `foreground`, and `primary` match the existing light/dark token tests before any widget is mounted.
