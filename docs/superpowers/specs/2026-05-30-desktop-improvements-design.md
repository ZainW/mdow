# Desktop App Improvements Design

Seven improvements to the Electron desktop app, ordered bottom-up from quick wins to major refactors. Each improvement includes a proof method to verify the change.

## Approach

Bottom-up: start with low-risk changes (dead code, silent errors, error boundary), then medium (cache eviction, folder updates), then big refactors (store split, MarkdownView decomposition). Proof is built into each step.

## 1. Remove Dead Code

### Items to remove

1. **`attachFileWatcher` wrapper** (`file-service.ts:114-116`) — just calls `setActiveFileWatch` with no added logic. Replace all call sites with `setActiveFileWatch` directly.

2. **`_getMainWindow` parameter** (`file-service.ts:88-89`) — `setActiveFileWatch` takes an unused first parameter. Remove it from the signature and all call sites in `ipc.ts`.

3. **`store:add-recent` IPC handler** (`ipc.ts:196-203`) — never called from the renderer. The preload does not expose an `addRecent` method. Remove the handler and the `STORE_ADD_RECENT` entry from `IPC` in `shared/types.ts`.

4. **`store:prune-recents` IPC handler** (`ipc.ts:191`) — not exposed in preload API. `getRecents` already prunes internally. Remove the handler.

5. **Duplicate `TreeNode` interfaces** — defined in 6 places but `shared/types.ts` already exports the canonical version. Update imports in: `folder-service.ts`, `app-store.ts`, `FolderTree.tsx`, `CommandPalette.tsx`, `useFolderTree.ts`.

6. **Duplicate `getReadErrorType`** — defined in both `ErrorView.tsx:37-43` and `useOpenMarkdownFile.ts:6-13`. Extract to `lib/error-utils.ts` and import from both locations.

### Proof

- `grep` showing 0 references to removed code
- `wc -l` before/after on affected files
- `pnpm run typecheck` passes
- `pnpm run lint` passes

## 2. Fix Silent Error Swallowing

### Current problems

1. **`useAppInit.ts:52-54`** — CLI `openPath` failure silently caught. User launches with `--openPath=/bad/file` and sees empty app with no explanation.

2. **`useAppInit.ts:84-86`** — Session tab restore failures silently skipped. User's tabs disappear without feedback.

### Fixes

1. **CLI openPath failure** — When the `openPath` file fails to load, open an error tab:
   ```ts
   catch (err) {
     openErrorTab(openPath, { type: getReadErrorType(err), path: openPath })
   }
   ```

2. **Session restore failures** — Track failed tabs. After the restore loop, log a warning with the count and paths of failed tabs:
   ```ts
   const failedPaths: string[] = []
   for (const tab of orderedTabs) {
     try {
       const content = await window.api.readFile(tab.path)
       openTab({ path: tab.path, content })
     } catch {
       failedPaths.push(tab.path)
     }
   }
   if (failedPaths.length > 0) {
     console.warn(`Failed to restore ${failedPaths.length} session tab(s):`, failedPaths)
   }
   ```

### Proof

- Before: `grep` shows bare `catch {}` blocks with empty bodies
- After: each catch has explicit handling
- Manual test: launch with invalid `openPath`, verify error tab appears

## 3. Global Error Boundary

### Current

`ErrorBoundary` only wraps `MarkdownView` (`App.tsx:38-40`). If `Sidebar`, `TabBar`, `CommandPalette`, or any dialog throws, the entire app white-screens.

### Fix

Add a top-level `AppErrorBoundary` class component in `components/AppErrorBoundary.tsx`. Wrap `MainApp`'s return in `App.tsx:122-143` with it.

The boundary:
- Catches any render error outside the markdown view
- Shows a full-screen recovery UI with the error message and a "Reload" button
- The "Reload" button calls `window.location.reload()` as a last resort
- Logs to `console.error` for debugging

The existing `ErrorBoundary` around `MarkdownView` stays as a more specific, contextual boundary. The global one is the safety net.

### Proof

- Before: triggering `throw new Error()` in `Sidebar` crashes the app to white screen
- After: the global boundary catches it and shows a recovery UI with reload option

## 4. Cache Eviction with ocache

### Current

Two unbounded caches grow forever:

1. **`contentRenderCache`** in `markdown.ts:147` — `Map<string, RenderResult>` keyed by full markdown content string.
2. **`svgCache`** in `mermaid.ts:16` — `Map<string, string>` keyed by `id:theme:code`.

### Fix

Install `ocache` from the unjs ecosystem. Replace both manual caches with `defineCachedFunction`:

1. **`renderMarkdown`** — wrap with `defineCachedFunction`:
   - `maxAge: 3600` (1 hour TTL)
   - `getKey: (text) => hash(text)` (content hash as key)
   - Replaces the manual `contentRenderCache` Map

2. **`renderMermaidBlock`** — wrap with `defineCachedFunction`:
   - `maxAge: 3600` (1 hour TTL)
   - `getKey: (block, isDark) => cacheKey(block, isDark)` (reuse existing key function)
   - Replaces the manual `svgCache` Map

Both get automatic eviction, TTL expiry, and request deduplication (concurrent renders of the same content only execute once).

### Proof

- Before: render 100 unique documents, cache grows to 100 entries with no eviction
- After: entries auto-evict after TTL, concurrent renders are deduplicated
- Memory usage stays bounded over long sessions

## 5. Incremental Folder Updates

### Current

Every file add/unlink in a watched folder triggers a full `scanFolder()` (`folder-service.ts:165`), which re-reads the entire directory tree. For a 5000-file folder, this means re-scanning everything on every single file change.

### Fix

Instead of full rescan on every change, apply incremental updates to a cached tree:

1. **Cache the last scan result** per watched folder path in a `Map<string, ScanResult>`.
2. **On `add` event** (new markdown file): walk the cached tree to find the parent directory node, insert the new file node in sorted position. If the parent directory doesn't exist in the tree, create it.
3. **On `unlink` event** (deleted markdown file): walk the cached tree to find and remove the file node. Remove empty parent directories.
4. **On `addDir`/`unlinkDir`**: fall back to full rescan (directory structural changes are rarer and harder to patch incrementally).
5. **Debounce stays at 1000ms** — if multiple changes arrive in the window, batch them into one incremental update. If any `addDir`/`unlinkDir` is in the batch, do a full rescan instead.

### Proof

- Before: add a file to a 1000-file folder, measure `scanFolder` time (full rescan)
- After: measure incremental insert time
- Expect 10-100x faster for large folders
- Tree structure remains correct (verified by comparing incremental result vs full rescan result)

## 6. Split Zustand Store

### Current

Single 421-line `app-store.ts` managing tabs, sidebar, folder tree, headings, UI dialogs, zoom, theme, typography, and auto-update. All in one `create()` call.

### Fix

Split into 4 domain slices using Zustand's slice pattern (single store, multiple slice creators):

1. **`tabSlice`** (~120 lines) — `tabs`, `activeTabId`, `openingPath`, `renderCache`, all tab actions
2. **`uiSlice`** (~80 lines) — `sidebarOpen`, `sidebarMode`, `wideMode`, `commandPaletteOpen`, `searchOpen`, `shortcutsDialogOpen`, `settingsOpen`, `docHeadings`, `activeHeadingId`
3. **`folderSlice`** (~20 lines) — `openFolderPath`, `folderTree`, `folderTreeTruncated`
4. **`settingsSlice`** (~80 lines) — `zoomLevel`, `theme`, `autoUpdateEnabled`, `contentFont`, `codeFont`, `fontSize`, `lineHeight`, all settings actions

File structure:
```
store/
  app-store.ts        — combines slices, exports useAppStore
  slices/
    tab-slice.ts
    ui-slice.ts
    folder-slice.ts
    settings-slice.ts
```

The `useAppStore` hook still works the same way for consumers — `useAppStore((s) => s.tabs)` continues to work. The split is internal only.

Side effects (calls to `window.api` in zoom/theme/font setters) move to a `useEffect`-based subscription pattern: the slice just updates state, and a separate `useSettingsEffects` hook in `App.tsx` watches for changes and calls `window.api`. This decouples the store from Electron.

### Proof

- Before: `wc -l app-store.ts` = 421
- After: no file exceeds ~130 lines
- Each slice is independently testable without mocking `window.api`
- All existing selectors continue to work unchanged

## 7. Decompose MarkdownView

### Current

`MarkdownView.tsx` is 551 lines handling: markdown rendering, render state machine, scroll restoration, scroll persistence, heading observation, mermaid theme sync, copy-to-clipboard delegation, link interception, search integration, and layout.

### Fix

Extract into focused hooks and sub-components:

**Hooks:**

1. **`useMarkdownRender`** (~60 lines) — the `renderReducer`, render effect, cache lookup, render version tracking. Returns `{ renderResult, renderError, isRendering, retry }`.
2. **`useScrollRestoration`** (~30 lines) — scroll-to-position on tab switch (`useLayoutEffect`), debounced scroll persistence (`useEffect` with scroll listener).
3. **`useHeadingObserver`** (~25 lines) — `IntersectionObserver` for heading elements, updates `activeHeadingId` in store.
4. **`useMermaidThemeSync`** (~20 lines) — `MutationObserver` on `<html>` class, re-renders mermaid blocks on theme change.
5. **`useContentClickHandlers`** (~50 lines) — copy-code delegation and link interception event listeners on the content container.

**Sub-components (extract to files):**

- `CodeBlock` → `components/markdown/CodeBlock.tsx`
- `MermaidBlock` → `components/markdown/MermaidBlock.tsx`
- `AlertCallout` → `components/markdown/AlertCallout.tsx`
- `TableWrap` → `components/markdown/TableWrap.tsx`
- `TaskCheckbox` → `components/markdown/TaskCheckbox.tsx`
- `createMarkdownComponents` → `components/markdown/components.ts`

### Proof

- Before: `wc -l MarkdownView.tsx` = 551
- After: ~120 lines
- Each hook is independently testable
- No behavior changes — all existing functionality preserved
