# Editing Mode — Design Spec

**Date:** 2026-04-25
**Status:** Approved (brainstorm)
**Owner:** Zain

## Summary

Add a per-tab editing mode to Mdow. The editing experience follows Obsidian's "Live Preview" model: a single unified surface that renders markdown elements inline as the user types, with the underlying file remaining plain markdown text on disk.

The current comark-based renderer is replaced with a Tiptap-based renderer used for **both** read and edit modes. Toggling between modes flips Tiptap's `editable` flag — same DOM, same scroll position, no visual jump.

## Goals

- Users can edit markdown documents directly in Mdow without losing the calm reading experience.
- Read mode and edit mode look visually identical for the same content.
- Files on disk stay plain markdown. External tools (git, other editors, sync clients) keep working.
- Editing is opt-in per tab so reference documents stay in read mode while working documents are edited.

## Non-goals (this work)

- Block-based editing (Notion-style). Slash commands are a v2 concern.
- Rename and delete. Out of scope; can be addressed later as independent file-management work.
- Image paste / drag-drop, math (KaTeX), footnote UI, link auto-complete. v1.1 / v2.
- Mobile / web build targets. Desktop only.

## Product decisions (settled in brainstorm)

| Decision               | Choice                                                                |
| ---------------------- | --------------------------------------------------------------------- |
| Editing model          | Obsidian-style live preview (text-first, decorations rendered inline) |
| Mode UX                | Read-first, edit on demand. `Cmd+E` toggles per-tab mode.             |
| Save model             | Auto-save, debounced (~500ms after typing stops). No dirty indicator. |
| Scope                  | Edit existing files + create new files. No rename/delete in v1.       |
| Editor library         | Tiptap (ProseMirror under the hood)                                   |
| Renderer for read mode | Same Tiptap instance with `editable: false` (replaces comark)         |

The Tiptap choice was made knowing the round-trip fidelity risk; mitigations are described under "Markdown round-trip strategy" below.

## Architecture

### Component layout

- **New** `apps/desktop/src/renderer/src/components/Editor.tsx` — Tiptap `EditorContent` wrapper. Owns the editor instance, mode prop, auto-save effect, scroll-spy effect, and search integration.
- **New** `apps/desktop/src/renderer/src/lib/editor/` — directory holding Tiptap extensions:
  - `extensions/code-block-shiki.ts` — Shiki-highlighted code blocks (replaces today's Shiki integration).
  - `extensions/mermaid-block.ts` — node view for `mermaid` fenced blocks.
  - `extensions/frontmatter.ts` — passthrough node for `---...---` frontmatter.
  - `extensions/html-passthrough.ts` — passthrough for raw HTML blocks/inline.
  - `extensions/heading-ids.ts` — adds `id` attributes to headings for breadcrumb scroll-spy.
  - `serializer.ts` — markdown serializer config (built on `prosemirror-markdown` with custom rules for the passthrough nodes).
  - `parser.ts` — markdown-it-based parser configuration consumed by Tiptap.
- **Removed** `apps/desktop/src/renderer/src/lib/markdown.ts` (comark renderer) and `MarkdownView.tsx`. Their roles are absorbed by `Editor.tsx`.
- **Updated** `App.tsx` — renders `<Editor tab={activeTab} />` instead of `<MarkdownView />`. The mode toggle keybinding lives here.

### State

`Tab` in `app-store.ts` gains:

```ts
interface Tab {
  // ...existing fields
  mode: 'read' | 'edit'
  lastDiskWriteAt?: number // timestamp of our last write, used to suppress watcher echoes
}
```

New actions:

- `toggleTabMode(tabId)` — flips `mode` between read and edit.
- `setTabMode(tabId, mode)` — explicit set (used when a new file opens in edit mode).
- `markTabWritten(tabId, timestamp)` — records `lastDiskWriteAt`.

### IPC additions

In `apps/desktop/src/main/ipc.ts` and `preload/index.ts`:

- `writeFile(path: string, content: string): Promise<void>` — writes UTF-8 content. Errors surface to the renderer.
- `createFile(folderPath: string | null, suggestedName?: string): Promise<{ path: string } | null>` — if `folderPath` is provided, creates a uniquely-named file in that folder (`Untitled.md`, `Untitled-2.md`, ...). If `null`, shows an OS save dialog. Returns `null` if the user cancels the dialog.

The existing file-service watchers stay as-is, but `writeFile` records the path+timestamp so the watcher can suppress its own echo (described below).

## Mode model & UX

- New tabs open in `read` mode by default.
- Files created via `Cmd+N` open in `edit` mode.
- `Cmd+E` toggles the active tab's mode. Toggling is instant; no save prompt (auto-save has already persisted any changes).
- Tab title in `TabBar.tsx` shows a small pencil icon when the tab is in edit mode. No dirty dot — auto-save means nothing is ever unsaved for more than 500ms.
- Switching tabs preserves each tab's mode independently.
- The `Cmd+E` shortcut and a "Toggle edit mode" command in the command palette both bind to the same action.

## Auto-save

- A React effect inside `Editor.tsx` subscribes to Tiptap's `onUpdate`.
- On update, schedule a debounced (500ms) write:
  1. Serialize the editor doc to markdown via `lib/editor/serializer.ts`.
  2. Compare against the last-known disk content for the tab. Skip the write if unchanged.
  3. Call `window.api.writeFile(tab.path, content)`.
  4. Record `lastDiskWriteAt = Date.now()` on the tab.
- The Zustand store keeps the buffer text in `tab.content` to stay consistent with how reads work today.
- If the write fails, surface a toast (or banner — exact UI TBD during implementation, but it must not be a blocking modal). The editor stays in its current state; nothing is lost from memory.

## File-watcher conflict policy

The watcher fires on every disk change including our own. Policy:

- When `writeFile` resolves, record `lastDiskWriteAt` for that path.
- When the watcher fires for a path:
  - If `Date.now() - lastDiskWriteAt < 1000ms`, treat it as our own echo and ignore.
  - Else if the tab is in `read` mode: reload content (current behavior).
  - Else if the tab is in `edit` mode and the disk content differs from the editor's current serialized content: show a non-blocking banner inside the tab — _"This file was changed on disk. \[Reload\] \[Keep my version\]"_. "Reload" replaces the editor content with disk; "Keep" dismisses and triggers a write so the editor's version becomes canonical.
  - Else (edit mode but content matches): silent.

## Markdown round-trip strategy

This is the highest-risk decision in the design. Tiptap is tree-based, and parsing markdown → tree → serializing back to markdown is lossy in the general case. Mitigations:

### Parser & serializer

- Parse with `markdown-it` (the same family used by comark today) to produce tokens, then convert to a ProseMirror doc using a Tiptap-friendly adapter.
- Serialize via `prosemirror-markdown`'s `MarkdownSerializer`, configured with explicit rules for every node type we accept.
- The serializer is **closed-world**: any node it doesn't know how to write becomes a passthrough source node (see below). It will never silently drop content.

### Passthrough nodes

Constructs that we cannot reliably round-trip through tree manipulation are stored as opaque source-preserving nodes:

| Construct                 | Read mode                                      | Edit mode                                           |
| ------------------------- | ---------------------------------------------- | --------------------------------------------------- |
| Frontmatter (`---...---`) | Collapsed grey panel showing key/value preview | Expandable to raw source; edits write back verbatim |
| Mermaid (` ```mermaid `)  | Rendered diagram (current behavior)            | Click to reveal source; click out renders again     |
| Raw HTML block            | Rendered HTML (sanitized as today)             | Source view in monospace                            |
| Inline HTML               | Source verbatim                                | Source verbatim                                     |
| Math (`$...$`, `$$...$$`) | Source verbatim (v1) — KaTeX in v2             | Source verbatim                                     |
| Footnotes (`[^1]`)        | Source verbatim (v1)                           | Source verbatim                                     |

### CI round-trip test

- Build a corpus of representative real-world markdown in `apps/desktop/src/renderer/src/lib/editor/__fixtures__/` (a dozen+ files: a README with badges and code, a blog post with images and links, a technical doc with mermaid + frontmatter, an `.mdx`-ish file, a doc with raw HTML, a doc with a complex table).
- Test: for every fixture, parse → serialize and assert the output is byte-for-byte identical to the input.
- Add fixtures whenever real-world content reveals a new failure mode. Each fix is either a serializer rule, a new passthrough node, or a documented limitation.
- This test gates merges of changes to `lib/editor/`.

### Failure mode

If a file contains content the parser cannot lossless-roundtrip, the file opens normally in read mode, but the edit button is disabled with a tooltip — _"This file uses markdown features Mdow can't safely edit yet."_ This is a fallback we expect to use rarely.

## New file (`Cmd+N`)

- Bound globally in `App.tsx`'s keydown handler.
- If `openFolderPath` is set: call `createFile(openFolderPath)`. Main picks the first available `Untitled-N.md` name and creates an empty file. The renderer opens it as a new tab in `edit` mode.
- If `openFolderPath` is null: call `createFile(null)`. Main shows an OS save dialog defaulting to `Untitled.md`. On confirm, creates the file at the chosen path and returns it. The renderer opens it in `edit` mode. On cancel, no-op.
- Added to the menu (`menu.ts`) under File → New, and to the command palette.

## Search

The current `useDocumentSearch` hook walks rendered HTML. With Tiptap, search becomes a ProseMirror decoration plugin:

- A new `lib/editor/extensions/search.ts` extension exposes `setQuery(query)`, `next()`, `prev()`, and emits `matchCount` / `currentIndex`.
- `SearchBar.tsx` is updated to talk to this extension instead of operating on HTML.
- Behavior is unchanged from the user's perspective: `Cmd+F` opens the bar, matches highlight, arrows cycle.

## Heading scroll-spy & breadcrumb

- The `heading-ids` extension adds `id` attributes to heading nodes deterministically (slugify on first render, stable across re-renders).
- `MarkdownView`'s `IntersectionObserver` for scroll-spy is moved into `Editor.tsx` and observes the rendered heading DOM.
- `DocumentBreadcrumb.tsx` keeps its current contract — it consumes `docHeadings` and `activeHeadingId` from the store, set by `Editor.tsx` on render.

## Performance considerations

- Tiptap + extensions: ~200KB gzipped. comark and its dependencies are removed. Net delta should be roughly comparable; will be measured during implementation.
- For large docs (>50KB), Tiptap's incremental rendering is fine; ProseMirror's transactions are well-optimized for this scale.
- Auto-save debounce of 500ms is tuned to feel instant on small docs without thrashing disk on rapid typing.

## Scope tiers

### v1 (this work)

- Tiptap renderer used for both modes
- `Cmd+E` mode toggle, per-tab mode state
- Auto-save with file-watcher conflict banner
- Headings, bold, italic, strikethrough, inline code, lists (ordered/unordered/task), links, blockquote, horizontal rule, code blocks (Shiki), Mermaid passthrough, frontmatter passthrough, HTML passthrough
- `Cmd+N` new file flow
- Round-trip CI test with real-world fixture corpus
- Menu and command palette entries

### v1.1 (separate work)

- Image inline rendering in editor
- Table editing UI (Tiptap has a table extension; integrating cleanly is its own task)
- Keyboard shortcut hints / formatting toolbar (TBD design)
- Markdown shortcut input rules (`#` becomes heading, `*` triggers bullet, etc.)

### v2 (separate work)

- Slash commands
- Drag-drop image paste with auto file storage
- Math (KaTeX) rendering
- Footnotes UI
- Link auto-complete to other docs in the open folder

## Risks & open questions

- **Round-trip fidelity is the single largest risk.** The CI fixture corpus is the safety net. If it starts catching too many cases we can't fix, we revisit the choice of Tiptap.
- **Bundle size and startup time** — to be measured during implementation; if Tiptap regresses cold start meaningfully, we lazy-load it for first edit-mode entry while keeping read mode synchronous.
- **Mermaid in edit mode** — exact UX of "click to reveal source, click out to render" needs polish during implementation. Likely behaviors: clicking inside the rendered SVG enters source mode for that block, clicking outside (or pressing `Esc`) exits.
- **Frontmatter UX** — should the collapsed panel show key/value chips or just "Frontmatter (3 keys)"? Decide during implementation.
- **Existing Shiki setup** — we currently render Shiki at parse time into HTML. With Tiptap, Shiki runs inside a node view. Need to confirm the existing theme/language list ships fine.

## Test plan

- Unit tests for the markdown parser/serializer (round-trip fixtures, individual node types).
- Unit tests for the conflict-resolution logic (own echo vs. external change vs. matching content).
- Unit tests for new IPC handlers (`writeFile`, `createFile` including unique-name generation).
- Component tests for `Editor.tsx` (mode toggle, auto-save debounce, search integration).
- Manual checklist for v1: open existing file, toggle to edit, type, watch auto-save fire, switch tab, return, mode preserved; create new file in folder; create new file with no folder open (save dialog); external edit reload banner; Mermaid edit/render flip.
