# Desktop App Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply 7 improvements to the desktop app: remove dead code, fix silent errors, add global error boundary, add cache eviction, implement incremental folder updates, split Zustand store, and decompose MarkdownView.

**Architecture:** Bottom-up approach — low-risk changes first (dead code, errors, error boundary), then medium (caches, folder updates), then big refactors (store, MarkdownView). Each improvement is independently verifiable.

**Tech Stack:** Electron, React 19, Zustand, Vitest, ocache (unjs), TypeScript

**Verification commands (run after each task):**
- `pnpm run typecheck` — type-check all workspaces
- `pnpm run lint` — oxlint across all workspaces
- `pnpm run test` — run all tests

---

## File Structure

### New files to create:
- `apps/desktop/src/renderer/src/lib/error-utils.ts` — shared `getReadErrorType` function
- `apps/desktop/src/renderer/src/components/AppErrorBoundary.tsx` — global error boundary
- `apps/desktop/src/renderer/src/store/slices/tab-slice.ts` — tab state slice
- `apps/desktop/src/renderer/src/store/slices/ui-slice.ts` — UI state slice
- `apps/desktop/src/renderer/src/store/slices/folder-slice.ts` — folder tree slice
- `apps/desktop/src/renderer/src/store/slices/settings-slice.ts` — settings slice
- `apps/desktop/src/renderer/src/hooks/useMarkdownRender.ts` — render state machine hook
- `apps/desktop/src/renderer/src/hooks/useScrollRestoration.ts` — scroll position hook
- `apps/desktop/src/renderer/src/hooks/useHeadingObserver.ts` — heading intersection hook
- `apps/desktop/src/renderer/src/hooks/useMermaidThemeSync.ts` — theme sync hook
- `apps/desktop/src/renderer/src/hooks/useContentClickHandlers.ts` — click delegation hook
- `apps/desktop/src/renderer/src/components/markdown/CodeBlock.tsx` — code block component
- `apps/desktop/src/renderer/src/components/markdown/MermaidBlock.tsx` — mermaid block component
- `apps/desktop/src/renderer/src/components/markdown/AlertCallout.tsx` — alert callout component
- `apps/desktop/src/renderer/src/components/markdown/TableWrap.tsx` — table wrapper component
- `apps/desktop/src/renderer/src/components/markdown/TaskCheckbox.tsx` — task checkbox component
- `apps/desktop/src/renderer/src/components/markdown/components.ts` — component factory

### Files to modify:
- `apps/desktop/src/main/file-service.ts` — remove dead code
- `apps/desktop/src/main/ipc.ts` — remove dead handlers, update call sites
- `apps/desktop/src/main/folder-service.ts` — incremental updates, remove duplicate TreeNode
- `apps/desktop/src/shared/types.ts` — remove dead IPC constants
- `apps/desktop/src/renderer/src/hooks/useAppInit.ts` — fix silent errors
- `apps/desktop/src/renderer/src/hooks/useOpenMarkdownFile.ts` — use shared error util
- `apps/desktop/src/renderer/src/components/ErrorView.tsx` — use shared error util
- `apps/desktop/src/renderer/src/components/FolderTree.tsx` — use shared TreeNode
- `apps/desktop/src/renderer/src/components/CommandPalette.tsx` — use shared TreeNode
- `apps/desktop/src/renderer/src/hooks/useFolderTree.ts` — use shared TreeNode
- `apps/desktop/src/renderer/src/store/app-store.ts` — split into slices
- `apps/desktop/src/renderer/src/lib/markdown.ts` — ocache integration
- `apps/desktop/src/renderer/src/lib/mermaid.ts` — ocache integration
- `apps/desktop/src/renderer/src/App.tsx` — add global error boundary
- `apps/desktop/src/renderer/src/components/MarkdownView.tsx` — decompose

---

## Task 1: Extract Shared Error Utility

**Files:**
- Create: `apps/desktop/src/renderer/src/lib/error-utils.ts`
- Modify: `apps/desktop/src/renderer/src/components/ErrorView.tsx:37-43`
- Modify: `apps/desktop/src/renderer/src/hooks/useOpenMarkdownFile.ts:6-13`

- [ ] **Step 1: Create the shared error utility**

Create `apps/desktop/src/renderer/src/lib/error-utils.ts`:

```ts
import type { ErrorType } from '../../../shared/types'

export function getReadErrorType(error: unknown): ErrorType {
  if (error instanceof Error) {
    if (error.message === 'not-found') return 'not-found'
    if (error.message === 'permission-denied') return 'permission-denied'
    if (error.message === 'read-error') return 'read-error'
  }
  return 'read-error'
}
```

- [ ] **Step 2: Update ErrorView.tsx to import from shared util**

In `apps/desktop/src/renderer/src/components/ErrorView.tsx`, replace the local import and remove the local function:

Replace:
```ts
import { useAppStore, type FileError, type ErrorType } from '../store/app-store'
```
With:
```ts
import { useAppStore, type FileError } from '../store/app-store'
import { getReadErrorType } from '../lib/error-utils'
```

Remove the local `getReadErrorType` function (lines 37-43).

- [ ] **Step 3: Update useOpenMarkdownFile.ts to import from shared util**

In `apps/desktop/src/renderer/src/hooks/useOpenMarkdownFile.ts`, replace the local function:

Replace:
```ts
import { useAppStore, type ErrorType } from '../store/app-store'
```
With:
```ts
import { useAppStore } from '../store/app-store'
import { getReadErrorType } from '../lib/error-utils'
```

Remove the local `getReadErrorType` function (lines 6-13).

- [ ] **Step 4: Run verification**

Run: `pnpm run typecheck && pnpm run lint && pnpm run test`
Expected: All pass. Existing tests for `useOpenMarkdownFile` and `ErrorView` continue to pass.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/renderer/src/lib/error-utils.ts apps/desktop/src/renderer/src/components/ErrorView.tsx apps/desktop/src/renderer/src/hooks/useOpenMarkdownFile.ts
git commit -m "refactor: extract shared getReadErrorType to error-utils"
```

---

## Task 2: Remove Dead IPC Handlers and Code

**Files:**
- Modify: `apps/desktop/src/main/ipc.ts`
- Modify: `apps/desktop/src/main/file-service.ts`
- Modify: `apps/desktop/src/shared/types.ts`

- [ ] **Step 1: Remove `attachFileWatcher` wrapper and `_getMainWindow` parameter**

In `apps/desktop/src/main/file-service.ts`:

Remove the `attachFileWatcher` function (lines 114-116):
```ts
export function attachFileWatcher(getMainWindow: () => BrowserWindow | null, path: string): void {
  setActiveFileWatch(getMainWindow, path)
}
```

Remove the unused `_getMainWindow` parameter from `setActiveFileWatch` (lines 88-112). Change signature from:
```ts
export function setActiveFileWatch(
  _getMainWindow: () => BrowserWindow | null,
  filePath: string | null,
): void {
```
To:
```ts
export function setActiveFileWatch(filePath: string | null): void {
```

- [ ] **Step 2: Update ipc.ts call sites**

In `apps/desktop/src/main/ipc.ts`:

Remove the import of `attachFileWatcher`:
```ts
import {
  openFileDialog,
  readFileContent,
  unwatchFile,
  attachFileWatcher,
  setActiveFileWatch,
} from './file-service'
```
Becomes:
```ts
import {
  openFileDialog,
  readFileContent,
  unwatchFile,
  setActiveFileWatch,
} from './file-service'
```

Replace all `attachFileWatcher(() => win, ...)` calls with `setActiveFileWatch(...)`:
- Line 50: `attachFileWatcher(() => win, result.path)` → `setActiveFileWatch(result.path)`
- Line 62: `attachFileWatcher(() => win, resolved)` → `setActiveFileWatch(resolved)`

Replace all `setActiveFileWatch(() => win, ...)` calls with `setActiveFileWatch(...)`:
- Line 112: `setActiveFileWatch(() => win, null)` → `setActiveFileWatch(null)`
- Line 118: `setActiveFileWatch(() => win, resolved)` → `setActiveFileWatch(resolved)`
- Line 120: `setActiveFileWatch(() => win, null)` → `setActiveFileWatch(null)`

- [ ] **Step 3: Remove dead IPC handlers**

In `apps/desktop/src/main/ipc.ts`, remove these handlers:

Remove `store:prune-recents` handler (line 191):
```ts
ipcMain.handle('store:prune-recents', () => pruneRecents())
```

Remove `store:add-recent` handler (lines 196-203):
```ts
ipcMain.handle('store:add-recent', (_, filePath: string) => {
  try {
    const resolved = validateMarkdownPath(filePath)
    trackRecentFile(getMainWindow, resolved)
  } catch {
    // Ignore invalid paths
  }
})
```

Remove unused import of `pruneRecents` from `./store`:
```ts
import {
  getRecents,
  addRecent,
  getAppState,
  saveAppState,
  setLastFolder,
  pruneRecents,
} from './store'
```
Becomes:
```ts
import {
  getRecents,
  addRecent,
  getAppState,
  saveAppState,
  setLastFolder,
} from './store'
```

- [ ] **Step 4: Remove dead IPC constants**

In `apps/desktop/src/shared/types.ts`, remove from the `IPC` object:
```ts
STORE_PRUNE_RECENTS: 'store:prune-recents',
```

- [ ] **Step 5: Update file-service test**

In `apps/desktop/src/main/file-service.test.ts`, verify that the test imports still work. The test imports `watchFile`, `unwatchFile`, `unwatchAllFiles` — none of which are removed. No changes needed.

- [ ] **Step 6: Run verification**

Run: `pnpm run typecheck && pnpm run lint && pnpm run test`
Expected: All pass. The `ipc-channels.test.ts` may need updating if it checks for removed channels — verify and fix if needed.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "refactor: remove dead IPC handlers, unused params, and dead code"
```

---

## Task 3: Consolidate Duplicate TreeNode Interfaces

**Files:**
- Modify: `apps/desktop/src/main/folder-service.ts:7-12`
- Modify: `apps/desktop/src/renderer/src/store/app-store.ts:12-17`
- Modify: `apps/desktop/src/renderer/src/components/FolderTree.tsx:16-21`
- Modify: `apps/desktop/src/renderer/src/components/CommandPalette.tsx:19-24`
- Modify: `apps/desktop/src/renderer/src/hooks/useFolderTree.ts:5-10`

- [ ] **Step 1: Update folder-service.ts to import TreeNode from shared**

In `apps/desktop/src/main/folder-service.ts`, remove the local `TreeNode` interface (lines 7-12) and add the import:

```ts
import { type TreeNode, type ScanResult } from '../shared/types'
```

Remove the local `ScanResult` interface (lines 14-17) as well since it's also in `shared/types.ts`.

- [ ] **Step 2: Update app-store.ts to import TreeNode from shared**

In `apps/desktop/src/renderer/src/store/app-store.ts`, remove the local `TreeNode` interface (lines 12-17) and add the import:

```ts
import type { TreeNode } from '../../../shared/types'
```

- [ ] **Step 3: Update FolderTree.tsx to import TreeNode from shared**

In `apps/desktop/src/renderer/src/components/FolderTree.tsx`, remove the local `TreeNode` interface and add:

```ts
import type { TreeNode } from '../../../shared/types'
```

- [ ] **Step 4: Update CommandPalette.tsx to import TreeNode from shared**

In `apps/desktop/src/renderer/src/components/CommandPalette.tsx`, remove the local `TreeNode` interface and add:

```ts
import type { TreeNode } from '../../../shared/types'
```

- [ ] **Step 5: Update useFolderTree.ts to import TreeNode from shared**

In `apps/desktop/src/renderer/src/hooks/useFolderTree.ts`, remove the local `TreeNode` interface and add:

```ts
import type { TreeNode } from '../../../shared/types'
```

- [ ] **Step 6: Run verification**

Run: `pnpm run typecheck && pnpm run lint && pnpm run test`
Expected: All pass. No behavioral changes.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "refactor: consolidate duplicate TreeNode interfaces to shared types"
```

---

## Task 4: Fix Silent Error Swallowing in useAppInit

**Files:**
- Modify: `apps/desktop/src/renderer/src/hooks/useAppInit.ts`

- [ ] **Step 1: Fix CLI openPath error handling**

In `apps/desktop/src/renderer/src/hooks/useAppInit.ts`, replace the silent catch at lines 52-54:

```ts
        } catch {
          // Failed to load CLI path, fall back gracefully
        }
```

With:

```ts
        } catch (err) {
          openErrorTab(openPath, { type: getReadErrorType(err), path: openPath })
        }
```

Add the import at the top:
```ts
import { getReadErrorType } from '../lib/error-utils'
```

Add `openErrorTab` to the destructured store selectors (line 6):
```ts
  const openTab = useAppStore((s) => s.openTab)
  const openErrorTab = useAppStore((s) => s.openErrorTab)
  const setOpenFolder = useAppStore((s) => s.setOpenFolder)
```

Update the `useEffect` dependency array to include `openErrorTab`:
```ts
  }, [setOpenFolder, openTab, openErrorTab])
```

- [ ] **Step 2: Fix session restore error handling**

Replace the session restore loop at lines 80-87:

```ts
        for (const tab of orderedTabs) {
          try {
            const content = await window.api.readFile(tab.path)
            openTab({ path: tab.path, content })
          } catch {
            // Skip unreadable session tabs
          }
        }
```

With:

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

- [ ] **Step 3: Run verification**

Run: `pnpm run typecheck && pnpm run lint && pnpm run test`
Expected: All pass.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "fix: surface errors in useAppInit instead of silently swallowing"
```

---

## Task 5: Add Global Error Boundary

**Files:**
- Create: `apps/desktop/src/renderer/src/components/AppErrorBoundary.tsx`
- Modify: `apps/desktop/src/renderer/src/App.tsx`

- [ ] **Step 1: Create AppErrorBoundary component**

Create `apps/desktop/src/renderer/src/components/AppErrorBoundary.tsx`:

```tsx
import { Component, type ErrorInfo, type ReactNode } from 'react'
import { RotateCcw, TriangleAlert } from 'lucide-react'
import { Button } from './ui/button'
import { iconStroke } from '../lib/icons'

interface Props {
  children: ReactNode
}

interface State {
  hasError: boolean
  error: Error | null
}

export class AppErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false, error: null }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error }
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error('Application render error:', error, info.componentStack)
  }

  render(): ReactNode {
    if (this.state.hasError) {
      return (
        <div className="flex h-screen w-screen flex-col items-center justify-center gap-4 bg-background p-12 text-center">
          <TriangleAlert
            className="size-12 text-destructive"
            strokeWidth={iconStroke.default}
          />
          <div>
            <h1 className="text-xl font-semibold">Something went wrong</h1>
            <p className="mt-2 text-sm text-muted-foreground">
              The application encountered an unexpected error.
            </p>
            {this.state.error && (
              <p className="mt-3 max-w-lg font-mono text-xs text-muted-foreground/70">
                {this.state.error.message}
              </p>
            )}
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={() => window.location.reload()}
          >
            <RotateCcw className="mr-2 size-3.5" strokeWidth={iconStroke.default} />
            Reload application
          </Button>
        </div>
      )
    }
    return this.props.children
  }
}
```

- [ ] **Step 2: Wrap MainApp with AppErrorBoundary**

In `apps/desktop/src/renderer/src/App.tsx`, add the import:

```ts
import { AppErrorBoundary } from './components/AppErrorBoundary'
```

Wrap the `MainApp` function's return statement. Change:

```tsx
  return (
    <SidebarProvider>
```

To:

```tsx
  return (
    <AppErrorBoundary>
      <SidebarProvider>
```

And add the closing tag before the final `)`:

```tsx
      </SidebarProvider>
    </AppErrorBoundary>
  )
```

- [ ] **Step 3: Run verification**

Run: `pnpm run typecheck && pnpm run lint && pnpm run test`
Expected: All pass.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: add global AppErrorBoundary to catch app-wide render errors"
```

---

## Task 6: Cache Eviction with ocache

**Files:**
- Modify: `apps/desktop/src/renderer/src/lib/markdown.ts`
- Modify: `apps/desktop/src/renderer/src/lib/mermaid.ts`
- Modify: `apps/desktop/package.json` (add ocache dependency)

- [ ] **Step 1: Install ocache**

Run: `pnpm add ocache --filter desktop`

- [ ] **Step 2: Replace contentRenderCache in markdown.ts**

In `apps/desktop/src/renderer/src/lib/markdown.ts`:

Add import at top:
```ts
import { defineCachedFunction } from 'ocache'
```

Remove the manual cache Map and related functions (lines 147-155):
```ts
const contentRenderCache = new Map<string, RenderResult>()

export function getCachedMarkdownRender(content: string): RenderResult | undefined {
  return contentRenderCache.get(content)
}

export function clearMarkdownRenderCache(): void {
  contentRenderCache.clear()
}
```

Replace `renderMarkdown` with a cached version. Change the function to be internal and create a cached export:

Rename the existing `renderMarkdown` to `_renderMarkdown` (the actual implementation stays the same, just remove the cache check at the top):

```ts
async function _renderMarkdown(
  text: string,
  options?: { bypassCache?: boolean },
): Promise<RenderResult> {
  const tree = await parse(text)

  const mermaidBlocks: { id: string; code: string }[] = []
  let mermaidCounter = 0
  const headings: DocHeading[] = []
  const slugCounts = new Map<string, number>()

  function visit(node: ComarkNode): void {
    if (!isElement(node)) return

    const tag = node[0]
    const attrs = node[1]

    if (/^h[1-6]$/.test(tag)) {
      const headingText = getNodeText(node).trim()
      if (headingText) {
        let id = typeof attrs.id === 'string' ? attrs.id : ''
        if (!id) {
          const base = slugifyHeading(headingText)
          if (base) {
            const count = slugCounts.get(base) ?? 0
            slugCounts.set(base, count + 1)
            id = count === 0 ? base : `${base}-${count}`
            attrs.id = id
          }
        }
        if (id) headings.push({ level: Number(tag.slice(1)), text: headingText, id })
      }
    }

    if (tag === 'mermaid') {
      const code = typeof attrs.content === 'string' ? attrs.content : ''
      const id =
        typeof attrs.id === 'string' && attrs.id.length > 0
          ? attrs.id
          : `mermaid-${mermaidCounter++}`
      attrs.id = id
      appendClassName(node, 'mermaid mermaid-container')
      mermaidBlocks.push({ id, code })
    }

    for (const child of getChildren(node)) visit(child)
  }

  for (const node of tree.nodes) visit(node)

  const result: RenderResult = {
    tree,
    mermaidBlocks,
    headings,
    frontmatter: tree.frontmatter ?? {},
  }
  return result
}

export const renderMarkdown = defineCachedFunction(_renderMarkdown, {
  name: 'renderMarkdown',
  maxAge: 3600,
  getKey: (text: string) => text,
})
```

Update `getCachedMarkdownRender` to use the cached function's internal cache:
```ts
export function getCachedMarkdownRender(content: string): RenderResult | undefined {
  return renderMarkdown.cache?.get(content)
}
```

Note: Check the ocache API for how to access the internal cache. If `.cache` is not available, remove `getCachedMarkdownRender` and update callers to just call `renderMarkdown` (it will return cached results automatically).

Update `clearMarkdownRenderCache`:
```ts
export function clearMarkdownRenderCache(): void {
  renderMarkdown.invalidate?.()
}
```

Note: Check the ocache API for the exact invalidation method. Adjust accordingly.

- [ ] **Step 3: Replace svgCache in mermaid.ts**

In `apps/desktop/src/renderer/src/lib/mermaid.ts`:

Add import at top:
```ts
import { defineCachedFunction } from 'ocache'
```

Remove the manual `svgCache` Map (line 16):
```ts
const svgCache = new Map<string, string>()
```

Remove `clearMermaidSvgCache` and `getMermaidSvgCacheSize` functions (lines 47-53).

Create a cached version of the SVG rendering step. Extract the SVG generation into a cached function:

```ts
async function _generateMermaidSvg(blockId: string, code: string, isDark: boolean): Promise<string> {
  const mermaid = await loadMermaid()
  mermaid.initialize({
    startOnLoad: false,
    theme: isDark ? 'dark' : 'default',
    securityLevel: 'loose',
  })
  const { svg } = await mermaid.render(`${blockId}-svg`, code)
  return svg
}

const generateMermaidSvg = defineCachedFunction(_generateMermaidSvg, {
  name: 'mermaidSvg',
  maxAge: 3600,
  getKey: (blockId: string, _code: string, isDark: boolean) => `${blockId}:${isDark ? 'dark' : 'light'}`,
})
```

Update `renderMermaidBlock` to use the cached function:

```ts
export async function renderMermaidBlock(
  block: { id: string; code: string },
  isDark = document.documentElement.classList.contains('dark'),
): Promise<void> {
  if (!mermaidInitialized) return

  const el = document.getElementById(block.id)
  if (!el) return

  try {
    el.replaceChildren()
    const svg = await generateMermaidSvg(block.id, block.code, isDark)
    applySvgToElement(el, svg)
  } catch (e) {
    el.className = 'mermaid-error'
    el.removeAttribute('role')
    el.textContent = `Mermaid diagram error: ${e instanceof Error ? e.message : String(e)}`
    const errorSvg = document.getElementById(`d${block.id}-svg`)
    if (errorSvg) errorSvg.remove()
  }
}
```

- [ ] **Step 4: Update tests**

In `apps/desktop/src/renderer/src/lib/mermaid.test.ts`, update any tests that reference `clearMermaidSvgCache` or `getMermaidSvgCacheSize` — these functions are removed. The existing tests should still pass since they test rendering behavior, not cache internals.

In `apps/desktop/src/renderer/src/lib/markdown.test.ts`, update any tests that reference `clearMarkdownRenderCache` — adjust to use the new invalidation API.

- [ ] **Step 5: Run verification**

Run: `pnpm run typecheck && pnpm run lint && pnpm run test`
Expected: All pass. Cache entries now auto-evict after TTL.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "perf: replace unbounded caches with ocache TTL-based eviction"
```

---

## Task 7: Incremental Folder Updates

**Files:**
- Modify: `apps/desktop/src/main/folder-service.ts`
- Modify: `apps/desktop/src/main/folder-service.test.ts`

- [ ] **Step 1: Add tree mutation helpers to folder-service.ts**

Add these helper functions to `apps/desktop/src/main/folder-service.ts`:

```ts
function findParentDir(nodes: TreeNode[], dirPath: string): TreeNode | null {
  for (const node of nodes) {
    if (node.isDirectory && node.path === dirPath) return node
    if (node.isDirectory && node.children) {
      const found = findParentDir(node.children, dirPath)
      if (found) return found
    }
  }
  return null
}

function insertFileNode(nodes: TreeNode[], filePath: string, fileName: string, parentPath: string): boolean {
  const parent = parentPath === '' ? null : findParentDir(nodes, parentPath)
  const target = parent?.children ?? nodes

  if (target.some((n) => n.path === filePath)) return false

  const insertIndex = target.findIndex((n) => {
    if (n.isDirectory) return false
    return n.name.localeCompare(fileName) > 0
  })

  const newNode: TreeNode = { name: fileName, path: filePath, isDirectory: false }
  if (insertIndex === -1) {
    target.push(newNode)
  } else {
    target.splice(insertIndex, 0, newNode)
  }
  return true
}

function removeFileNode(nodes: TreeNode[], filePath: string): boolean {
  const index = nodes.findIndex((n) => n.path === filePath)
  if (index !== -1) {
    nodes.splice(index, 1)
    return true
  }
  for (const node of nodes) {
    if (node.isDirectory && node.children) {
      if (removeFileNode(node.children, filePath)) {
        if (node.children.length === 0) {
          const parentIndex = nodes.indexOf(node)
          nodes.splice(parentIndex, 1)
        }
        return true
      }
    }
  }
  return false
}
```

- [ ] **Step 2: Add cached tree state and incremental update logic**

Add a cache Map and update the `watchFolder` function:

```ts
const folderTreeCache = new Map<string, ScanResult>()
```

In `watchFolder`, replace the `handleChange` function to support incremental updates:

```ts
  let pendingChanges: Array<{ type: 'add' | 'unlink' | 'addDir' | 'unlinkDir'; path: string }> = []

  const flushChanges = () => {
    const changes = pendingChanges
    pendingChanges = []

    const cached = folderTreeCache.get(folderPath)
    const hasStructuralChange = changes.some((c) => c.type === 'addDir' || c.type === 'unlinkDir')

    if (!cached || hasStructuralChange) {
      void scanFolder(folderPath)
        .then((result) => {
          folderTreeCache.set(folderPath, result)
          onChange(result)
        })
        .catch(() => {})
      return
    }

    let modified = false
    for (const change of changes) {
      if (change.type === 'add') {
        const sep = change.path.includes('\\') ? '\\' : '/'
        const parts = change.path.split(sep)
        const fileName = parts.pop()!
        const parentPath = parts.join(sep)
        if (isMdFile(fileName)) {
          if (insertFileNode(cached.tree, change.path, fileName, parentPath === folderPath ? '' : parentPath)) {
            modified = true
          }
        }
      } else if (change.type === 'unlink') {
        if (removeFileNode(cached.tree, change.path)) {
          modified = true
        }
      }
    }

    if (modified) {
      onChange({ tree: cached.tree, truncated: cached.truncated })
    }
  }

  const handleChange = (type: 'add' | 'unlink' | 'addDir' | 'unlinkDir', path: string) => {
    pendingChanges.push({ type, path })
    if (watchState.debounceTimer) clearTimeout(watchState.debounceTimer)
    watchState.debounceTimer = setTimeout(() => {
      watchState.debounceTimer = null
      flushChanges()
    }, 1000)
  }
```

Update the watcher event handlers:

```ts
  watcher.on('add', (path) => {
    if (isMarkdownPath(path)) handleChange('add', path)
  })
  watcher.on('unlink', (path) => {
    if (isMarkdownPath(path)) handleChange('unlink', path)
  })
  watcher.on('addDir', (path) => handleChange('addDir', path))
  watcher.on('unlinkDir', (path) => handleChange('unlinkDir', path))
```

Also cache the initial scan result in `watchFolder` by adding after the watcher setup:
```ts
  void scanFolder(folderPath).then((result) => {
    folderTreeCache.set(folderPath, result)
  })
```

And clean up cache in `unwatchFolder`:
```ts
  folderTreeCache.delete(folderPath)
```

- [ ] **Step 3: Export helpers and write tests for incremental updates**

In `apps/desktop/src/main/folder-service.ts`, export the helpers:

```ts
export function insertFileNode(nodes: TreeNode[], filePath: string, fileName: string, parentPath: string): boolean {
```

```ts
export function removeFileNode(nodes: TreeNode[], filePath: string): boolean {
```

Add tests to `apps/desktop/src/main/folder-service.test.ts`:

```ts
import { scanFolder, insertFileNode, removeFileNode } from './folder-service'
import type { TreeNode } from '../shared/types'

describe('insertFileNode', () => {
  it('inserts a file at the root level in sorted position', () => {
    const nodes: TreeNode[] = [
      { name: 'a.md', path: '/root/a.md', isDirectory: false },
      { name: 'c.md', path: '/root/c.md', isDirectory: false },
    ]
    const result = insertFileNode(nodes, '/root/b.md', 'b.md', '')
    expect(result).toBe(true)
    expect(nodes).toHaveLength(3)
    expect(nodes[1].name).toBe('b.md')
  })

  it('inserts a file into a nested directory', () => {
    const nodes: TreeNode[] = [
      {
        name: 'docs',
        path: '/root/docs',
        isDirectory: true,
        children: [{ name: 'a.md', path: '/root/docs/a.md', isDirectory: false }],
      },
    ]
    const result = insertFileNode(nodes, '/root/docs/b.md', 'b.md', '/root/docs')
    expect(result).toBe(true)
    expect(nodes[0].children).toHaveLength(2)
    expect(nodes[0].children![1].name).toBe('b.md')
  })

  it('does not insert duplicates', () => {
    const nodes: TreeNode[] = [
      { name: 'a.md', path: '/root/a.md', isDirectory: false },
    ]
    const result = insertFileNode(nodes, '/root/a.md', 'a.md', '')
    expect(result).toBe(false)
    expect(nodes).toHaveLength(1)
  })
})

describe('removeFileNode', () => {
  it('removes a file from the root level', () => {
    const nodes: TreeNode[] = [
      { name: 'a.md', path: '/root/a.md', isDirectory: false },
      { name: 'b.md', path: '/root/b.md', isDirectory: false },
    ]
    const result = removeFileNode(nodes, '/root/a.md')
    expect(result).toBe(true)
    expect(nodes).toHaveLength(1)
    expect(nodes[0].name).toBe('b.md')
  })

  it('removes a file from a nested directory', () => {
    const nodes: TreeNode[] = [
      {
        name: 'docs',
        path: '/root/docs',
        isDirectory: true,
        children: [
          { name: 'a.md', path: '/root/docs/a.md', isDirectory: false },
          { name: 'b.md', path: '/root/docs/b.md', isDirectory: false },
        ],
      },
    ]
    const result = removeFileNode(nodes, '/root/docs/a.md')
    expect(result).toBe(true)
    expect(nodes[0].children).toHaveLength(1)
  })

  it('removes empty parent directories after file removal', () => {
    const nodes: TreeNode[] = [
      {
        name: 'empty-dir',
        path: '/root/empty-dir',
        isDirectory: true,
        children: [{ name: 'only.md', path: '/root/empty-dir/only.md', isDirectory: false }],
      },
    ]
    const result = removeFileNode(nodes, '/root/empty-dir/only.md')
    expect(result).toBe(true)
    expect(nodes).toHaveLength(0)
  })
})
```

- [ ] **Step 4: Run verification**

Run: `pnpm run typecheck && pnpm run lint && pnpm run test`
Expected: All pass. Existing `scanFolder` tests unchanged.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "perf: incremental folder updates instead of full rescans"
```

---

## Task 8: Split Zustand Store — Create Slices

**Files:**
- Create: `apps/desktop/src/renderer/src/store/slices/tab-slice.ts`
- Create: `apps/desktop/src/renderer/src/store/slices/ui-slice.ts`
- Create: `apps/desktop/src/renderer/src/store/slices/folder-slice.ts`
- Create: `apps/desktop/src/renderer/src/store/slices/settings-slice.ts`
- Modify: `apps/desktop/src/renderer/src/store/app-store.ts`

- [ ] **Step 1: Create tab-slice.ts**

Create `apps/desktop/src/renderer/src/store/slices/tab-slice.ts` with all tab-related state and actions extracted from `app-store.ts`. Include the `Tab` interface, `FileError` type, `ErrorType`, `selectActiveTab` selector, and all tab actions (`openTab`, `closeTab`, `closeOtherTabs`, `closeTabsToRight`, `closeAllTabs`, `reorderTabs`, `cycleTab`, `selectTabByIndex`, `setActiveTab`, `updateTabContent`, `updateTabScroll`, `setTabError`, `clearTabError`, `setOpeningPath`, `setRenderCache`, `openErrorTab`).

The slice exports a `TabSlice` interface and a `createTabSlice` function:

```ts
import type { StateCreator } from 'zustand'
import type { RenderResult } from '../../lib/markdown'

export interface Tab {
  id: string
  path: string
  content: string
  scrollPosition: number
  error?: FileError | null
}

export type ErrorType = 'not-found' | 'permission-denied' | 'deleted' | 'read-error'

export interface FileError {
  type: ErrorType
  path: string
}

export interface TabSlice {
  tabs: Tab[]
  activeTabId: string | null
  openingPath: string | null
  renderCache: Map<string, RenderResult>
  openTab: (file: { path: string; content: string }) => void
  openErrorTab: (path: string, error: FileError) => void
  closeTab: (tabId: string) => void
  closeOtherTabs: (tabId: string) => void
  closeTabsToRight: (tabId: string) => void
  closeAllTabs: () => void
  reorderTabs: (fromIndex: number, toIndex: number) => void
  cycleTab: (direction: 1 | -1) => void
  selectTabByIndex: (index: number) => void
  setActiveTab: (tabId: string) => void
  updateTabContent: (path: string, content: string) => void
  updateTabScroll: (tabId: string, scrollPosition: number) => void
  setTabError: (path: string, error: FileError) => void
  clearTabError: (tabId: string) => void
  setOpeningPath: (path: string | null) => void
  setRenderCache: (tabId: string, result: RenderResult | null) => void
}

export const createTabSlice: StateCreator<TabSlice, [], [], TabSlice> = (set) => ({
  tabs: [],
  activeTabId: null,
  openingPath: null,
  renderCache: new Map(),

  openTab: (file) =>
    set((state) => {
      const existing = state.tabs.find((t) => t.path === file.path)
      if (existing) {
        return {
          activeTabId: existing.id,
          tabs: state.tabs.map((t) =>
            t.id === existing.id ? { ...t, content: file.content, error: null } : t,
          ),
        }
      }
      const newTab: Tab = {
        id: crypto.randomUUID(),
        path: file.path,
        content: file.content,
        scrollPosition: 0,
      }
      const activeIndex = state.tabs.findIndex((t) => t.id === state.activeTabId)
      const insertIndex = activeIndex >= 0 ? activeIndex + 1 : state.tabs.length
      const tabs = [...state.tabs]
      tabs.splice(insertIndex, 0, newTab)
      return { tabs, activeTabId: newTab.id }
    }),

  openErrorTab: (path, error) =>
    set((state) => {
      const existing = state.tabs.find((t) => t.path === path)
      if (existing) {
        return {
          activeTabId: existing.id,
          tabs: state.tabs.map((t) => (t.id === existing.id ? { ...t, error } : t)),
        }
      }
      const newTab: Tab = { id: crypto.randomUUID(), path, content: '', scrollPosition: 0, error }
      const activeIndex = state.tabs.findIndex((t) => t.id === state.activeTabId)
      const insertIndex = activeIndex >= 0 ? activeIndex + 1 : state.tabs.length
      const tabs = [...state.tabs]
      tabs.splice(insertIndex, 0, newTab)
      return { tabs, activeTabId: newTab.id }
    }),

  closeTab: (tabId) =>
    set((state) => {
      const index = state.tabs.findIndex((t) => t.id === tabId)
      if (index === -1) return state
      unwatchPath(state.tabs[index].path)
      const tabs = state.tabs.filter((t) => t.id !== tabId)
      let activeTabId = state.activeTabId
      if (state.activeTabId === tabId) {
        if (tabs.length === 0) activeTabId = null
        else if (index < tabs.length) activeTabId = tabs[index].id
        else activeTabId = tabs[tabs.length - 1].id
      }
      return { tabs, activeTabId, renderCache: withoutRenderCache(state.renderCache, [tabId]) }
    }),

  closeOtherTabs: (tabId) =>
    set((state) => {
      const keep = state.tabs.find((t) => t.id === tabId)
      if (!keep) return state
      for (const t of state.tabs) {
        if (t.id !== tabId) unwatchPath(t.path)
      }
      const removedIds = state.tabs.filter((t) => t.id !== tabId).map((t) => t.id)
      return {
        tabs: [keep],
        activeTabId: tabId,
        renderCache: withoutRenderCache(state.renderCache, removedIds),
      }
    }),

  closeTabsToRight: (tabId) =>
    set((state) => {
      const index = state.tabs.findIndex((t) => t.id === tabId)
      if (index === -1) return state
      const tabs = state.tabs.slice(0, index + 1)
      const removed = state.tabs.slice(index + 1)
      for (const t of removed) unwatchPath(t.path)
      const stillActive = tabs.some((t) => t.id === state.activeTabId)
      return {
        tabs,
        activeTabId: stillActive ? state.activeTabId : tabId,
        renderCache: withoutRenderCache(state.renderCache, removed.map((t) => t.id)),
      }
    }),

  closeAllTabs: () =>
    set((state) => {
      for (const t of state.tabs) unwatchPath(t.path)
      return { tabs: [], activeTabId: null, renderCache: new Map() }
    }),

  reorderTabs: (fromIndex, toIndex) =>
    set((state) => {
      if (fromIndex === toIndex) return state
      if (fromIndex < 0 || fromIndex >= state.tabs.length) return state
      if (toIndex < 0 || toIndex > state.tabs.length) return state
      const tabs = [...state.tabs]
      const [moved] = tabs.splice(fromIndex, 1)
      const adjusted = toIndex > fromIndex ? toIndex - 1 : toIndex
      tabs.splice(adjusted, 0, moved)
      return { tabs }
    }),

  cycleTab: (direction) =>
    set((state) => {
      if (state.tabs.length === 0) return state
      const i = state.tabs.findIndex((t) => t.id === state.activeTabId)
      const next = (i + direction + state.tabs.length) % state.tabs.length
      return { activeTabId: state.tabs[next].id }
    }),

  selectTabByIndex: (index) =>
    set((state) => {
      if (index < 0 || index >= state.tabs.length) return state
      return { activeTabId: state.tabs[index].id }
    }),

  setActiveTab: (tabId) => set({ activeTabId: tabId }),

  updateTabContent: (path, content) =>
    set((state) => {
      const tab = state.tabs.find((t) => t.path === path)
      const renderCache = tab ? withoutRenderCache(state.renderCache, [tab.id]) : state.renderCache
      return {
        tabs: state.tabs.map((t) => (t.path === path ? { ...t, content } : t)),
        renderCache,
      }
    }),

  updateTabScroll: (tabId, scrollPosition) =>
    set((state) => ({
      tabs: state.tabs.map((t) => {
        if (t.id !== tabId || t.scrollPosition === scrollPosition) return t
        return { ...t, scrollPosition }
      }),
    })),

  setTabError: (path, error) =>
    set((state) => ({
      tabs: state.tabs.map((t) => (t.path === path ? { ...t, error } : t)),
    })),

  clearTabError: (tabId) =>
    set((state) => ({
      tabs: state.tabs.map((t) => (t.id === tabId ? { ...t, error: null } : t)),
    })),

  setOpeningPath: (path) => set({ openingPath: path }),

  setRenderCache: (tabId, result) =>
    set((state) => {
      const renderCache = new Map(state.renderCache)
      if (result === null) renderCache.delete(tabId)
      else renderCache.set(tabId, result)
      return { renderCache }
    }),
})
```

Also include the private helpers `unwatchPath` and `withoutRenderCache` in the file (not exported):

```ts
function unwatchPath(path: string): void {
  if (typeof window === 'undefined' || !window.api) return
  void window.api.unwatchFile(path)
}

function withoutRenderCache(
  renderCache: Map<string, RenderResult>,
  tabIds: string[],
): Map<string, RenderResult> {
  const next = new Map(renderCache)
  for (const tabId of tabIds) next.delete(tabId)
  return next
}
```

And export `selectActiveTab` from this file:

```ts
export const selectActiveTab = (s: TabSlice): Tab | null =>
  s.tabs.find((t) => t.id === s.activeTabId) ?? null
```

- [ ] **Step 2: Create ui-slice.ts**

Create `apps/desktop/src/renderer/src/store/slices/ui-slice.ts` with sidebar, wide mode, dialog states, and heading state:

```ts
import type { StateCreator } from 'zustand'
import type { DocHeading } from '../../lib/markdown'

export type SidebarMode = 'recents' | 'folder' | 'outline'

export interface UiSlice {
  sidebarOpen: boolean
  sidebarMode: SidebarMode
  toggleSidebar: () => void
  setSidebarMode: (mode: SidebarMode) => void
  wideMode: boolean
  toggleWideMode: () => void
  commandPaletteOpen: boolean
  setCommandPaletteOpen: (open: boolean) => void
  searchOpen: boolean
  setSearchOpen: (open: boolean) => void
  shortcutsDialogOpen: boolean
  setShortcutsDialogOpen: (open: boolean) => void
  settingsOpen: boolean
  setSettingsOpen: (open: boolean) => void
  docHeadings: DocHeading[]
  activeHeadingId: string | null
  setDocHeadings: (headings: DocHeading[]) => void
  setActiveHeadingId: (id: string | null) => void
}

export const createUiSlice: StateCreator<UiSlice, [], [], UiSlice> = (set) => ({
  sidebarOpen: true,
  sidebarMode: 'recents',
  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
  setSidebarMode: (mode) => {
    if (typeof window !== 'undefined' && window.api) {
      void window.api.saveAppState({ sidebarMode: mode })
    }
    set({ sidebarMode: mode })
  },
  wideMode: false,
  toggleWideMode: () =>
    set((state) => {
      const wideMode = !state.wideMode
      if (typeof window !== 'undefined' && window.api) {
        void window.api.saveAppState({ wideMode })
      }
      return { wideMode }
    }),
  commandPaletteOpen: false,
  setCommandPaletteOpen: (open) => set({ commandPaletteOpen: open }),
  searchOpen: false,
  setSearchOpen: (open) => set({ searchOpen: open }),
  shortcutsDialogOpen: false,
  setShortcutsDialogOpen: (open) => set({ shortcutsDialogOpen: open }),
  settingsOpen: false,
  setSettingsOpen: (open) => set({ settingsOpen: open }),
  docHeadings: [],
  activeHeadingId: null,
  setDocHeadings: (headings) => set({ docHeadings: headings }),
  setActiveHeadingId: (id) => set({ activeHeadingId: id }),
})
```

- [ ] **Step 3: Create folder-slice.ts**

Create `apps/desktop/src/renderer/src/store/slices/folder-slice.ts`:

```ts
import type { StateCreator } from 'zustand'
import type { TreeNode } from '../../../../shared/types'

export interface FolderSlice {
  openFolderPath: string | null
  folderTree: TreeNode[]
  folderTreeTruncated: boolean
  setOpenFolder: (path: string, tree: TreeNode[], truncated: boolean) => void
  setFolderTree: (tree: TreeNode[], truncated: boolean) => void
}

export const createFolderSlice: StateCreator<FolderSlice, [], [], FolderSlice> = (set) => ({
  openFolderPath: null,
  folderTree: [],
  folderTreeTruncated: false,
  setOpenFolder: (path, tree, truncated) =>
    set({ openFolderPath: path, folderTree: tree, folderTreeTruncated: truncated }),
  setFolderTree: (tree, truncated) => set({ folderTree: tree, folderTreeTruncated: truncated }),
})
```

- [ ] **Step 4: Create settings-slice.ts**

Create `apps/desktop/src/renderer/src/store/slices/settings-slice.ts`:

```ts
import type { StateCreator } from 'zustand'

export interface SettingsSlice {
  initialized: boolean
  zoomLevel: number
  zoomIn: () => void
  zoomOut: () => void
  resetZoom: () => void
  theme: string
  setTheme: (theme: string) => void
  autoUpdateEnabled: boolean
  setAutoUpdateEnabled: (enabled: boolean) => void
  contentFont: string
  codeFont: string
  fontSize: number
  lineHeight: number
  setContentFont: (font: string) => void
  setCodeFont: (font: string) => void
  setFontSize: (size: number) => void
  setLineHeight: (height: number) => void
}

export const createSettingsSlice: StateCreator<SettingsSlice, [], [], SettingsSlice> = (set) => ({
  initialized: false,
  zoomLevel: 100,
  zoomIn: () =>
    set((state) => {
      const next = Math.min(state.zoomLevel + 10, 200)
      void window.api.saveAppState({ zoomLevel: next })
      return { zoomLevel: next }
    }),
  zoomOut: () =>
    set((state) => {
      const next = Math.max(state.zoomLevel - 10, 60)
      void window.api.saveAppState({ zoomLevel: next })
      return { zoomLevel: next }
    }),
  resetZoom: () => {
    void window.api.saveAppState({ zoomLevel: 100 })
    return set({ zoomLevel: 100 })
  },
  theme: 'system',
  setTheme: (theme) => {
    void window.api.setTheme(theme)
    set({ theme })
  },
  autoUpdateEnabled: true,
  setAutoUpdateEnabled: (enabled) => {
    void window.api.saveAppState({ autoUpdateEnabled: enabled })
    void window.api.setAutoUpdateScheduling(enabled)
    set({ autoUpdateEnabled: enabled })
  },
  contentFont: 'inter',
  codeFont: 'geist-mono',
  fontSize: 15.5,
  lineHeight: 1.65,
  setContentFont: (font) => {
    void window.api.saveAppState({ contentFont: font })
    set({ contentFont: font })
  },
  setCodeFont: (font) => {
    void window.api.saveAppState({ codeFont: font })
    set({ codeFont: font })
  },
  setFontSize: (size) => {
    void window.api.saveAppState({ fontSize: size })
    set({ fontSize: size })
  },
  setLineHeight: (height) => {
    void window.api.saveAppState({ lineHeight: height })
    set({ lineHeight: height })
  },
})
```

- [ ] **Step 5: Rewrite app-store.ts to combine slices**

Replace `apps/desktop/src/renderer/src/store/app-store.ts` with:

```ts
import { create } from 'zustand'
import { createTabSlice, type TabSlice, type Tab, type FileError, type ErrorType } from './slices/tab-slice'
import { createUiSlice, type UiSlice, type SidebarMode } from './slices/ui-slice'
import { createFolderSlice, type FolderSlice } from './slices/folder-slice'
import { createSettingsSlice, type SettingsSlice } from './slices/settings-slice'

export type { Tab, FileError, ErrorType, SidebarMode }

type AppStore = TabSlice & UiSlice & FolderSlice & SettingsSlice

export const useAppStore = create<AppStore>()((...args) => ({
  ...createTabSlice(...args),
  ...createUiSlice(...args),
  ...createFolderSlice(...args),
  ...createSettingsSlice(...args),
}))

export const selectActiveTab = (s: AppStore): Tab | null =>
  s.tabs.find((t) => t.id === s.activeTabId) ?? null

function saveSession(tabs: Tab[], activeTabId: string | null): void {
  if (typeof window === 'undefined' || !window.api) return
  const activeTab = tabs.find((t) => t.id === activeTabId)
  void window.api.saveAppState({
    sessionTabs: tabs.map((t) => ({ path: t.path })),
    sessionActiveTabPath: activeTab?.path ?? null,
  })
}

function getSessionKey(tabs: Tab[], activeTabId: string | null): string {
  const activeTab = tabs.find((t) => t.id === activeTabId)
  return JSON.stringify({ paths: tabs.map((t) => t.path), activePath: activeTab?.path ?? null })
}

useAppStore.subscribe((state, prev) => {
  if (getSessionKey(state.tabs, state.activeTabId) !== getSessionKey(prev.tabs, prev.activeTabId)) {
    saveSession(state.tabs, state.activeTabId)
  }
})
```

- [ ] **Step 6: Update all imports across the codebase**

Search for all files importing from `../store/app-store` or `./store/app-store` and verify they still work. The exports (`useAppStore`, `selectActiveTab`, `Tab`, `FileError`, `ErrorType`, `SidebarMode`) are all re-exported from the new `app-store.ts`, so no consumer changes are needed.

- [ ] **Step 7: Update existing store tests**

In `apps/desktop/src/renderer/src/store/app-store.test.ts`, update the `beforeEach` reset to match the new combined store shape. The existing `useAppStore.setState({...})` calls should still work since the store shape is identical.

- [ ] **Step 8: Run verification**

Run: `pnpm run typecheck && pnpm run lint && pnpm run test`
Expected: All pass. Store behavior is identical.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "refactor: split Zustand store into domain-specific slices"
```

---

## Task 9: Decompose MarkdownView — Extract Sub-Components

**Files:**
- Create: `apps/desktop/src/renderer/src/components/markdown/CodeBlock.tsx`
- Create: `apps/desktop/src/renderer/src/components/markdown/MermaidBlock.tsx`
- Create: `apps/desktop/src/renderer/src/components/markdown/AlertCallout.tsx`
- Create: `apps/desktop/src/renderer/src/components/markdown/TableWrap.tsx`
- Create: `apps/desktop/src/renderer/src/components/markdown/TaskCheckbox.tsx`
- Create: `apps/desktop/src/renderer/src/components/markdown/components.ts`

- [ ] **Step 1: Create CodeBlock.tsx**

Create `apps/desktop/src/renderer/src/components/markdown/CodeBlock.tsx` with the `CodeBlock` component extracted from `MarkdownView.tsx` lines 80-115. Include imports for `Check`, `Copy`, `iconSize`, `iconStroke`.

- [ ] **Step 2: Create MermaidBlock.tsx**

Create `apps/desktop/src/renderer/src/components/markdown/MermaidBlock.tsx` with the `MermaidBlock` component extracted from `MarkdownView.tsx` lines 117-155. Include imports for `useEffect`, `useRef`, `useState`, `renderMermaidBlock`.

- [ ] **Step 3: Create AlertCallout.tsx**

Create `apps/desktop/src/renderer/src/components/markdown/AlertCallout.tsx` with the `AlertCallout` component and `ALERT_TYPES` constant from `MarkdownView.tsx` lines 61-78.

- [ ] **Step 4: Create TableWrap.tsx**

Create `apps/desktop/src/renderer/src/components/markdown/TableWrap.tsx` with the `TableWrap` component from `MarkdownView.tsx` lines 157-163.

- [ ] **Step 5: Create TaskCheckbox.tsx**

Create `apps/desktop/src/renderer/src/components/markdown/TaskCheckbox.tsx` with the `TaskCheckbox` component from `MarkdownView.tsx` lines 165-185.

- [ ] **Step 6: Create components.ts factory**

Create `apps/desktop/src/renderer/src/components/markdown/components.ts` with:
- The `createMarkdownComponents` function from `MarkdownView.tsx` lines 187-214
- Helper functions `resolveRelativePath` and `rewriteImageSrc` from lines 44-59
- The `MarkdownContent` memo component from lines 216-225

Import all sub-components from their new files. Export `MarkdownContent` for use in `MarkdownView.tsx`.

```ts
import { memo, useMemo } from 'react'
import { ComarkRenderer } from '@comark/react'
import type { RenderResult } from '../../lib/markdown'
import { detectSep } from '../../lib/path-utils'
import CodeBlock from './CodeBlock'
import MermaidBlock from './MermaidBlock'
import AlertCallout, { ALERT_TYPES } from './AlertCallout'
import TableWrap from './TableWrap'
import TaskCheckbox from './TaskCheckbox'
import type { AnchorHTMLAttributes, ImgHTMLAttributes } from 'react'

export function resolveRelativePath(href: string, docPath: string): string {
  const sep = detectSep(docPath)
  const dirParts = docPath.split(/[/\\]/).slice(0, -1)
  for (const segment of href.split(/[/\\]/)) {
    if (segment === '..') dirParts.pop()
    else if (segment === '.' || segment === '') continue
    else dirParts.push(segment)
  }
  return dirParts.join(sep)
}

function rewriteImageSrc(src: string, docPath: string): string {
  if (/^(https?:|data:|mdow-local:|blob:)/i.test(src)) return src
  const resolved = resolveRelativePath(src, docPath)
  return `mdow-local://local/${encodeURIComponent(resolved)}`
}

function createMarkdownComponents(docPath: string) {
  const alertComponents = Object.fromEntries(
    ALERT_TYPES.map((type) => [
      type,
      (props: React.HTMLAttributes<HTMLDivElement>) => <AlertCallout type={type} {...props} />,
    ]),
  )

  return {
    pre: CodeBlock,
    mermaid: MermaidBlock,
    math: ComarkMath,
    table: TableWrap,
    input: TaskCheckbox,
    img: ({ src, alt, ...props }: ImgHTMLAttributes<HTMLImageElement>) => (
      <img src={src ? rewriteImageSrc(src, docPath) : src} alt={alt ?? ''} loading="lazy" {...props} />
    ),
    a: ({ children, ...props }: AnchorHTMLAttributes<HTMLAnchorElement>) => (
      <a {...props}>{children ?? props.href}</a>
    ),
    ...alertComponents,
  }
}

export const MarkdownContent = memo(function MarkdownContent({
  result,
  docPath,
}: {
  result: RenderResult
  docPath: string
}) {
  const components = useMemo(() => createMarkdownComponents(docPath), [docPath])
  return <ComarkRenderer tree={result.tree} components={components} />
})
```

- [ ] **Step 7: Update MarkdownView.tsx imports**

In `MarkdownView.tsx`, remove all inline component definitions and import from the new files:

```ts
import { MarkdownContent } from './markdown/components'
```

Remove from `MarkdownView.tsx`:
- `AlertCallout` component and `ALERT_TYPES` constant
- `CodeBlock` component
- `MermaidBlock` component
- `TableWrap` component
- `TaskCheckbox` component
- `createMarkdownComponents` function
- `MarkdownContent` memo component (now in `components.ts`)
- `resolveRelativePath` and `rewriteImageSrc` helper functions
- Unused imports: `ComarkRenderer`, `ComarkMath`, `memo`, `useMemo`, `Check`, `Copy`, `iconSize`, `iconStroke`, `AnchorHTMLAttributes`, `CSSProperties`, `HTMLAttributes`, `ImgHTMLAttributes`, `InputHTMLAttributes`

Keep: `resolveRelativePath` import from `./markdown/components` (needed by `useContentClickHandlers` in Task 10).

- [ ] **Step 8: Run verification**

Run: `pnpm run typecheck && pnpm run lint && pnpm run test`
Expected: All pass. No behavioral changes.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "refactor: extract markdown sub-components to separate files"
```

---

## Task 10: Decompose MarkdownView — Extract Hooks

**Files:**
- Create: `apps/desktop/src/renderer/src/hooks/useMarkdownRender.ts`
- Create: `apps/desktop/src/renderer/src/hooks/useScrollRestoration.ts`
- Create: `apps/desktop/src/renderer/src/hooks/useHeadingObserver.ts`
- Create: `apps/desktop/src/renderer/src/hooks/useMermaidThemeSync.ts`
- Create: `apps/desktop/src/renderer/src/hooks/useContentClickHandlers.ts`
- Modify: `apps/desktop/src/renderer/src/components/MarkdownView.tsx`

- [ ] **Step 1: Create useMarkdownRender hook**

Create `apps/desktop/src/renderer/src/hooks/useMarkdownRender.ts` extracting the render state machine from `MarkdownView.tsx`:

- The `RenderUi` interface (lines 227-231)
- The `RenderAction` type (lines 233-238)
- The `renderReducer` function (lines 240-255)
- The render effect (lines 295-336)
- The heading sync effect (lines 338-344)

Returns: `{ renderResult, renderError, isRendering, renderVersion, retry }`

Parameters: `{ tabId: string, content: string, retryKey: number }`

- [ ] **Step 2: Create useScrollRestoration hook**

Create `apps/desktop/src/renderer/src/hooks/useScrollRestoration.ts` extracting:

- The scroll-to-position `useLayoutEffect` (lines 423-431)
- The debounced scroll persistence `useEffect` (lines 406-421)

Parameters: `{ scrollRef: RefObject<HTMLDivElement>, tabId: string, scrollPosition: number, updateTabScroll: (id: string, pos: number) => void }`

- [ ] **Step 3: Create useHeadingObserver hook**

Create `apps/desktop/src/renderer/src/hooks/useHeadingObserver.ts` extracting:

- The `IntersectionObserver` for headings (lines 382-404)

Parameters: `{ scrollRef: RefObject<HTMLDivElement>, contentRef: RefObject<HTMLDivElement>, renderResult: RenderResult | null }`

- [ ] **Step 4: Create useMermaidThemeSync hook**

Create `apps/desktop/src/renderer/src/hooks/useMermaidThemeSync.ts` extracting:

- The mermaid init effect (lines 352-355)
- The mermaid blocks ref sync (lines 357-359)
- The MutationObserver for theme changes (lines 361-380)

Parameters: `{ renderResult: RenderResult | null }`

- [ ] **Step 5: Create useContentClickHandlers hook**

Create `apps/desktop/src/renderer/src/hooks/useContentClickHandlers.ts` extracting:

- The copy-code click delegation (lines 437-449)
- The link click interception (lines 452-480)

Parameters: `{ contentRef: RefObject<HTMLDivElement>, tabPath: string, renderResult: RenderResult | null, onOpenMarkdownLink?: (path: string) => void }`

- [ ] **Step 6: Rewrite MarkdownView.tsx to use hooks**

Replace the inline logic in `MarkdownView.tsx` with hook calls:

```tsx
export function MarkdownView({ tab, onOpenMarkdownLink }: MarkdownViewProps) {
  const contentRef = useRef<HTMLDivElement>(null)
  const scrollRef = useRef<HTMLDivElement>(null)
  const [searchQuery, setSearchQuery] = useState('')

  const { wideMode, zoomLevel, contentFont, codeFont, fontSize, lineHeight } = useAppStore(
    useShallow((s) => ({
      wideMode: s.wideMode,
      zoomLevel: s.zoomLevel,
      contentFont: s.contentFont,
      codeFont: s.codeFont,
      fontSize: s.fontSize,
      lineHeight: s.lineHeight,
    })),
  )

  const { searchOpen, setSearchOpen, updateTabScroll } = useAppStore(
    useShallow((s) => ({
      searchOpen: s.searchOpen,
      setSearchOpen: s.setSearchOpen,
      updateTabScroll: s.updateTabScroll,
    })),
  )

  const [retryKey, setRetryKey] = useState(0)

  const { renderResult, renderError, isRendering, renderVersion } = useMarkdownRender({
    tabId: tab.id,
    content: tab.content,
    retryKey,
  })

  useScrollRestoration({
    scrollRef,
    tabId: tab.id,
    scrollPosition: tab.scrollPosition,
    updateTabScroll,
  })

  useHeadingObserver({ scrollRef, contentRef, renderResult })
  useMermaidThemeSync({ renderResult })
  useContentClickHandlers({
    contentRef,
    tabPath: tab.path,
    renderResult,
    onOpenMarkdownLink,
  })

  const { matchCount, currentIndex, next, prev, clear } = useDocumentSearch(
    contentRef,
    searchOpen ? searchQuery : '',
    renderVersion,
  )

  const handleCloseSearch = () => {
    setSearchOpen(false)
    setSearchQuery('')
    clear()
  }

  const handleRetry = () => setRetryKey((key) => key + 1)

  return (
    <div ref={scrollRef} className="group/content relative flex-1 overflow-y-auto">
      {searchOpen && (
        <SearchBar
          matchCount={matchCount}
          currentIndex={currentIndex}
          onNext={next}
          onPrev={prev}
          onClose={handleCloseSearch}
          onQueryChange={setSearchQuery}
        />
      )}
      <div
        ref={contentRef}
        id={`tabpanel-${tab.id}`}
        role="tabpanel"
        aria-labelledby={`tab-${tab.id}`}
        aria-busy={isRendering}
        className="mx-auto px-12 py-8 text-foreground markdown-body transition-[max-width] duration-200 ease-[cubic-bezier(0.23,1,0.32,1)]"
        style={
          {
            maxWidth: wideMode ? '100%' : '48rem',
            '--md-content-font': getContentFontFamily(contentFont),
            '--md-code-font': getCodeFontFamily(codeFont),
            '--md-font-size': `${fontSize * (zoomLevel / 100)}px`,
            '--md-line-height': String(lineHeight),
          } as CSSProperties
        }
      >
        {renderError ? (
          <div className="rounded-lg border border-destructive/20 bg-destructive/5 p-4 text-sm">
            <p className="text-destructive">This document could not be rendered.</p>
            <button
              type="button"
              className="mt-3 rounded-md border border-border bg-background px-3 py-1.5 text-xs font-medium text-foreground hover:bg-muted"
              onClick={handleRetry}
            >
              Try again
            </button>
          </div>
        ) : renderResult ? (
          <div key={renderVersion} className="document-content-in">
            <MarkdownContent result={renderResult} docPath={tab.path} />
          </div>
        ) : isRendering ? (
          <DocumentSkeleton />
        ) : null}
      </div>
      <ZoomIndicator />
    </div>
  )
}
```

- [ ] **Step 7: Run verification**

Run: `pnpm run typecheck && pnpm run lint && pnpm run test`
Expected: All pass. `MarkdownView.startup.test.tsx` and other tests continue to pass.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "refactor: decompose MarkdownView into focused hooks and components"
```

---

## Task 11: Final Verification

- [ ] **Step 1: Run full verification suite**

Run: `pnpm run typecheck && pnpm run lint && pnpm run fmt:check && pnpm run test`
Expected: All pass.

- [ ] **Step 2: Verify line count improvements**

Run:
```bash
wc -l apps/desktop/src/renderer/src/components/MarkdownView.tsx
wc -l apps/desktop/src/renderer/src/store/app-store.ts
wc -l apps/desktop/src/main/folder-service.ts
```

Expected:
- `MarkdownView.tsx`: ~120 lines (was 551)
- `app-store.ts`: ~30 lines (was 421, logic moved to slices)
- No single slice file exceeds ~130 lines

- [ ] **Step 3: Verify dead code removal**

Run:
```bash
grep -r "attachFileWatcher" apps/desktop/src/
grep -r "store:add-recent" apps/desktop/src/
grep -r "store:prune-recents" apps/desktop/src/
```

Expected: No results (all dead code removed).

- [ ] **Step 4: Verify no duplicate types**

Run:
```bash
grep -rn "interface TreeNode" apps/desktop/src/
grep -rn "function getReadErrorType" apps/desktop/src/
```

Expected: `TreeNode` defined only in `shared/types.ts`. `getReadErrorType` defined only in `lib/error-utils.ts`.

- [ ] **Step 5: Verify cache eviction**

Run existing cache-related tests:
```bash
pnpm run --filter desktop test -- -t "cache"
pnpm run --filter desktop test -- -t "mermaid"
```

Expected: Tests pass. Caches now have TTL-based eviction.

- [ ] **Step 6: Final commit (if any fixups needed)**

```bash
git add -A
git commit -m "chore: final cleanup after desktop improvements"
```
