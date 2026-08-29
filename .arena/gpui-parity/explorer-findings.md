# How-explorer digest

Verified by reading the code in this session. Use this plus the named source files. Do not treat uncited claims as fact.

## Question

How does the GPUI reader own state, and how does Electron persist chrome, so a native parity layer can attach without replacing the document renderer?

## GPUI state

`MdowApp` is the only window entity. `AppModel` is a plain struct it owns: `tabs`, `workspace`, `workspace_error`. Chrome flags (`sidebar_open`, `wide_mode`), reader transients, watcher, and theme live on `MdowApp`. Nothing is persisted. No search, palette, settings, recents, or overlay stack.

Attach point for document/workspace work is `AppModel`. Attach point for chrome is new typed state beside `MdowApp`, not new fields scattered as Electron-style booleans.

Source: `apps/gpui/src/app.rs`, `tabs.rs`, `workspace.rs`, `watcher.rs`, `theme.rs`, `main.rs`, `actions.rs`.

## GPUI renderer

`load_source` → `parse_document` → `prepare_document` → `render_document`. Native GPUI only. No webview.

Strike is parsed then flattened. Alerts are generic quotes. Footnote refs are `[^{label}]`. Footnote defs leak as paragraphs. Mermaid is a code fence. Math flags are off. HTML is inert `RawText`. `ParsedDocument::plain_text` and painted `inline_layout` disagree on SoftBreak.

Search has no hook. Natural extension is `InlineStyleRange` / `TextRun.background_color` / `StyledText::with_highlights`. New blocks need `DocumentBlock`, parser match, `render_block`, margins, and optional `collect_code_blocks`.

Source: `apps/gpui/src/document.rs`, `syntax.rs`, `ui/reader.rs`.

## Electron chrome

Zustand slices plus `electron-store` (`config.json` under `userData`). Renderer never imports the store. IPC is `store:get-state`, `store:save-state`, `store:get-recents`.

Persisted: recents (max 20, missing paths pruned), lastFolder, session tab paths, active path, theme, fonts, scale, reading width, zoom, wideMode, sidebarMode, windowBounds, autoUpdate. Companion keys exist on a separate API and stay out of this run.

Not persisted: `sidebarOpen` (always starts true), overlay flags, palette/find query strings, split view / pane ids, per-tab scroll, outline headings, folder tree, render cache. `AppState` lists split-session fields that are never written.

Session is paths, not a snapshot. Restore re-reads files, mints new tab ids, scroll 0, no error tabs for failed paths. `openPath` on a window skips restore but can still overwrite the shared session. `lastFolder` restore does not switch sidebar mode to folder. `readFile` always `addRecent`, so restore rewrites recents order.

Overlays are independent booleans. Find is markdown-only DOM marks after 120ms. HTML is a sandboxed iframe. Palette is renderer `Cmd+K` only. Several shortcuts fire from both the menu and renderer keydown.

Source: `apps/desktop/src/main/store.ts`, `src/renderer/src/store/*`, `hooks/useAppInit.ts`, `hooks/useDocumentSearch.ts`, `hooks/useAppBindings.ts`, `components/{Sidebar,CommandPalette,SettingsDialog,SearchBar,HtmlView}.tsx`.

## Constraints that survived review

Keep the pulldown-cmark renderer. Do not adopt a webview. Exclusive overlay. Parse persistence at a boundary. Electron visual contract stays. Companion stays out.
