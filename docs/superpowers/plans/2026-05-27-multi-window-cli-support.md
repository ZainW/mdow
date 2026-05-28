# Multi-Window CLI Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow users to open folders and files in new independent window instances via a global CLI utility `mdow <path>` utilizing a memory-efficient, single-instance multi-window architecture.

**Architecture:** Maintain a Set of active `BrowserWindow` instances in the main process, passing opening paths through URL query parameters (`?openPath=...`). Update the frontend initialization to read this query parameter, load the corresponding folder or file, and bypass general session restoration.

**Tech Stack:** Electron, Node.js, React, Zustand, TypeScript

---

### Task 1: Refactor Folder Watcher for Multi-Window Support

Currently, `folder-service.ts` tracks a single folder watcher as a file-level global variable. We need to support multiple active folder watchers (one per open folder) keyed by folder path.

**Files:**
- Modify: `apps/desktop/src/main/folder-service.ts`

- [ ] **Step 1.1: Read folder-service.ts to confirm context**
Read the file to make sure we replace the correct global variables.

- [ ] **Step 1.2: Refactor watchFolder and unwatchFolder**
Replace the global `folderWatcher` and `folderDebounceTimer` with a `Map<string, FolderWatchState>` and update `unwatchFolder` to accept an optional `folderPath` parameter.

```typescript
interface FolderWatchState {
  watcher: FSWatcher
  debounceTimer: ReturnType<typeof setTimeout> | null
}

const activeFolderWatchers = new Map<string, FolderWatchState>()

export function watchFolder(folderPath: string, onChange: (result: ScanResult) => void): void {
  // Clean up any existing watcher for this specific path first
  unwatchFolder(folderPath)

  const watcher = watch(folderPath, {
    ignoreInitial: true,
    ignored: (path) => {
      const base = path.split(/[/\\]/).pop() ?? ''
      if (base.startsWith('.')) return true
      return IGNORED_DIRS.has(base)
    },
    depth: MAX_DEPTH,
  })

  const watchState: FolderWatchState = {
    watcher,
    debounceTimer: null,
  }

  const handleChange = () => {
    if (watchState.debounceTimer) clearTimeout(watchState.debounceTimer)
    watchState.debounceTimer = setTimeout(() => {
      watchState.debounceTimer = null
      void scanFolder(folderPath)
        .then((result) => {
          onChange(result)
        })
        .catch(() => {
          // Folder might have been deleted
        })
    }, 1000)
  }

  const handleFileChange = (path: string) => {
    if (isMarkdownPath(path)) handleChange()
  }

  watcher.on('add', handleFileChange)
  watcher.on('unlink', handleFileChange)
  watcher.on('addDir', handleChange)
  watcher.on('unlinkDir', handleChange)

  activeFolderWatchers.set(folderPath, watchState)
}

export function unwatchFolder(folderPath?: string): void {
  if (folderPath) {
    const state = activeFolderWatchers.get(folderPath)
    if (state) {
      if (state.debounceTimer) clearTimeout(state.debounceTimer)
      void state.watcher.close()
      activeFolderWatchers.delete(folderPath)
    }
  } else {
    for (const [path, state] of activeFolderWatchers.entries()) {
      if (state.debounceTimer) clearTimeout(state.debounceTimer)
      void state.watcher.close()
    }
    activeFolderWatchers.clear()
  }
}
```

- [ ] **Step 1.3: Verify typecheck passes**
Run type-checking on the desktop workspace:
Run: `pnpm run --filter desktop typecheck`
Expected: PASS

---

### Task 2: Refactor IPC Handlers to Target Senders Dynamically

In `ipc.ts`, replace hardcoded calls to the single `getMainWindow()` with dynamic lookups using `BrowserWindow.fromWebContents(event.sender)`. Also broadcast file changes to all open windows.

**Files:**
- Modify: `apps/desktop/src/main/ipc.ts`

- [ ] **Step 2.1: Refactor file change broadcast inside file watcher callback**
Modify the file watch handler or `attachFileWatcher`/`setActiveFileWatch` in `ipc.ts` (or `file-service.ts`) so that file change events are broadcast to all windows.
Wait, let's update `setActiveFileWatch` in `apps/desktop/src/main/file-service.ts` to broadcast to all open windows.

```typescript
export function setActiveFileWatch(
  getMainWindow: () => BrowserWindow | null,
  filePath: string | null,
): void {
  if (activeWatchPath && activeWatchPath !== filePath) {
    unwatchFile(activeWatchPath)
  }

  activeWatchPath = filePath

  if (!filePath) return

  watchFile(filePath, (event) => {
    // Broadcast to all windows
    const allWindows = BrowserWindow.getAllWindows()
    for (const win of allWindows) {
      if (win.isDestroyed()) continue
      if (event.type === 'changed') {
        win.webContents.send('file:changed', { path: filePath, content: event.content })
      } else if (event.type === 'deleted') {
        win.webContents.send('file:deleted', filePath)
      }
    }
  })
}
```

- [ ] **Step 2.2: Refactor folder IPC handlers to target the specific sender**
Update handlers inside `apps/desktop/src/main/ipc.ts` for folder operations to use `BrowserWindow.fromWebContents(event.sender)` to target the correct window.

```typescript
  ipcMain.handle('folder:open-dialog', async (event) => {
    const win = BrowserWindow.fromWebContents(event.sender)
    if (!win) return null
    const result = await openFolderDialog(win)
    if (result) {
      setLastFolder(result.path)
      registerAllowedPath(result.path)
      setupFolderWatcher(() => win, result.path)
    }
    return result
  })

  ipcMain.handle('folder:open-path', async (event, folderPath: string) => {
    const win = BrowserWindow.fromWebContents(event.sender)
    if (!win) throw new Error('no-window')
    try {
      const resolved = validatePath(folderPath)
      const stats = await stat(resolved)
      if (!stats.isDirectory()) {
        throw new Error('not-a-directory')
      }
      const result = await scanFolder(resolved)
      setLastFolder(resolved)
      registerAllowedPath(resolved)
      setupFolderWatcher(() => win, resolved)
      return { path: resolved, ...result }
    } catch (err: unknown) {
      // (Keep existing error handler blocks intact)
    }
  })

  ipcMain.handle('folder:read-tree', async (event, folderPath: string) => {
    const win = BrowserWindow.fromWebContents(event.sender)
    if (!win) throw new Error('no-window')
    try {
      const resolved = validatePath(folderPath)
      const result = await scanFolder(resolved)
      setLastFolder(resolved)
      registerAllowedPath(resolved)
      setupFolderWatcher(() => win, resolved)
      return result
    } catch (err: unknown) {
      // (Keep existing error handler blocks intact)
    }
  })
```

- [ ] **Step 2.3: Verify typecheck passes**
Run: `pnpm run --filter desktop typecheck`
Expected: PASS

---

### Task 3: Refactor index.ts for Multi-Window Support

Refactor `apps/desktop/src/main/index.ts` to support tracking multiple windows, focusing existing ones for open paths, and forwarding target paths via URL queries.

**Files:**
- Modify: `apps/desktop/src/main/index.ts`

- [ ] **Step 3.1: Declare the windows tracking Set and windowPaths map**
```typescript
const windows = new Set<BrowserWindow>()
const windowPaths = new Map<BrowserWindow, string>()
```

- [ ] **Step 3.2: Refactor getMainWindow() to return focused/active window**
```typescript
function getMainWindow(): BrowserWindow | null {
  const focused = BrowserWindow.getFocusedWindow()
  if (focused && windows.has(focused)) {
    return focused
  }
  if (windows.size > 0) {
    return Array.from(windows)[0]
  }
  return null
}
```

- [ ] **Step 3.3: Update createWindow to accept optional targetPath**
Load target path through query params:

```typescript
function createWindow(targetPath?: string): void {
  const savedBounds = getWindowBounds()

  const win = new BrowserWindow({
    width: savedBounds?.width ?? 1000,
    height: savedBounds?.height ?? 700,
    x: savedBounds?.x,
    y: savedBounds?.y,
    minWidth: 600,
    minHeight: 400,
    show: false,
    ...getWindowChromeOptions(),
    ...(isLinux
      ? {
          icon: is.dev
            ? join(__dirname, '../../resources/icon.png')
            : join(process.resourcesPath, 'icon.png'),
        }
      : {}),
    webPreferences: {
      preload: join(__dirname, '../preload/index.mjs'),
      sandbox: false,
      contextIsolation: true,
      nodeIntegration: false,
    },
  })

  windows.add(win)
  if (targetPath) {
    windowPaths.set(win, validatePath(targetPath))
  }

  if (savedBounds?.isMaximized && windows.size === 1) {
    win.maximize()
  }

  setupWebContentsSecurity(win)

  win.on('ready-to-show', () => {
    win.show()
  })

  const saveBounds = () => {
    if (win && !win.isDestroyed()) {
      saveWindowBounds(win.getBounds(), win.isMaximized())
    }
  }
  win.on('resized', saveBounds)
  win.on('moved', saveBounds)
  win.on('maximize', saveBounds)
  win.on('unmaximize', saveBounds)

  const handleNativeThemeUpdate = () => {
    if (win && !win.isDestroyed()) {
      applyWindowChrome(win)
      win.webContents.send('theme:changed', nativeTheme.shouldUseDarkColors)
    }
  }
  nativeTheme.on('updated', handleNativeThemeUpdate)

  win.webContents.on('did-finish-load', () => {
    win.webContents.send('theme:changed', nativeTheme.shouldUseDarkColors)
    if (!targetPath && windows.size === 1) {
      openFileFromArgv(process.argv, win)
    }
  })

  if (is.dev && process.env['ELECTRON_RENDERER_URL']) {
    const url = process.env['ELECTRON_RENDERER_URL']
    const queryStr = targetPath ? `?openPath=${encodeURIComponent(targetPath)}` : ''
    void win.loadURL(url + queryStr)
  } else {
    const htmlPath = join(__dirname, '../renderer/index.html')
    void win.loadFile(htmlPath, { query: targetPath ? { openPath: targetPath } : undefined })
  }

  win.on('closed', () => {
    nativeTheme.off('updated', handleNativeThemeUpdate)
    const folder = windowPaths.get(win)
    if (folder) unwatchFolder(folder)
    windowPaths.delete(win)
    windows.delete(win)
    
    if (windows.size === 0) {
      unwatchAllFiles()
      unwatchFolder()
      clearAllowedPaths()
      if (!isMac) app.quit()
    }
  })
}
```

- [ ] **Step 3.4: Refactor openFileFromArgv and app second-instance handler**
Allow `openFileFromArgv` to open in a target window. If a second instance receives a path, check if it's already open; if so focus that window, otherwise call `createWindow(targetPath)`.

```typescript
function openFileFromArgv(argv: string[], win: BrowserWindow): void {
  const filePath = argv.find(isMarkdownPath)
  if (filePath) {
    const resolved = validateMarkdownPath(filePath)
    void (async () => {
      try {
        const content = await readFileContent(resolved)
        addRecent(resolved)
        registerAllowedFile(resolved)
        if (isMac) {
          app.addRecentDocument(resolved)
        }
        attachFileWatcher(() => win, resolved)
        win.webContents.send('file:opened', { path: resolved, content })
      } catch {
        // Invalid file path
      }
    })()
  }
}
```

Under `second-instance`:
```typescript
  app.on('second-instance', (_event, argv) => {
    const targetPath = argv.find((arg) => {
      // Find arguments that exist as folders or files on disk
      try {
        const resolved = validatePath(arg)
        return resolved && !arg.startsWith('-')
      } catch {
        return false
      }
    })

    if (targetPath) {
      const resolved = validatePath(targetPath)
      // Check if a window is already open to this path
      let existingWin: BrowserWindow | null = null
      for (const [win, path] of windowPaths.entries()) {
        if (path === resolved) {
          existingWin = win
          break
        }
      }

      if (existingWin) {
        if (existingWin.isMinimized()) existingWin.restore()
        existingWin.focus()
      } else {
        createWindow(resolved)
      }
    } else {
      // Default: focus first window
      const win = getMainWindow()
      if (win) {
        if (win.isMinimized()) win.restore()
        win.focus()
      }
    }
  })
```

- [ ] **Step 3.5: Verify typecheck passes**
Run: `pnpm run --filter desktop typecheck`
Expected: PASS

---

### Task 4: Refactor Frontend Initializer for URL Query Parameter

Update `useAppInit.ts` to detect and load the `openPath` query parameter on startup, bypassing general saved session restoration.

**Files:**
- Modify: `apps/desktop/src/renderer/src/hooks/useAppInit.ts`

- [ ] **Step 4.1: Refactor useAppInit logic**
Read query parameter `openPath`. If present, check if it's a directory or a file, open it directly, and set initial state.

```typescript
import { useEffect } from 'react'
import type { AppState } from '../../../shared/types'
import { useAppStore } from '../store/app-store'

export function useAppInit(): void {
  const openTab = useAppStore((s) => s.openTab)
  const setOpenFolder = useAppStore((s) => s.setOpenFolder)

  useEffect(() => {
    const urlParams = new URLSearchParams(window.location.search)
    const openPath = urlParams.get('openPath')

    void window.api.getAppState().then(async (state: AppState) => {
      const patch: Record<string, unknown> = {}

      if (typeof state.wideMode === 'boolean') patch.wideMode = state.wideMode
      if (
        state.sidebarMode === 'recents' ||
        state.sidebarMode === 'folder' ||
        state.sidebarMode === 'outline'
      ) {
        patch.sidebarMode = state.sidebarMode
      }
      if (state.zoomLevel && state.zoomLevel !== 100) patch.zoomLevel = state.zoomLevel
      if (state.theme) patch.theme = state.theme
      if (typeof state.autoUpdateEnabled === 'boolean') {
        patch.autoUpdateEnabled = state.autoUpdateEnabled
      }
      if (state.contentFont) patch.contentFont = state.contentFont
      if (state.codeFont) patch.codeFont = state.codeFont
      if (state.fontSize) patch.fontSize = state.fontSize
      if (state.lineHeight) patch.lineHeight = state.lineHeight

      if (Object.keys(patch).length > 0) {
        useAppStore.setState(patch)
      }

      // If openPath query parameter is specified, open it directly
      if (openPath) {
        try {
          const stat = await window.api.statFile(openPath)
          if (stat.exists) {
            if (stat.isDirectory) {
              const scan = await window.api.readFolderTree(openPath)
              setOpenFolder(openPath, scan.tree, scan.truncated)
              useAppStore.setState({ sidebarMode: 'folder' })
            } else if (stat.isFile) {
              const content = await window.api.readFile(openPath)
              openTab({ path: openPath, content })
            }
          }
        } catch {
          // Failed to load CLI path, fall back gracefully
        }
        useAppStore.setState({ initialized: true })
        return
      }

      // Fallback: Standard state restoration
      if (state.lastFolder) {
        void window.api
          .readFolderTree(state.lastFolder)
          .then((scan) => {
            setOpenFolder(state.lastFolder!, scan.tree, scan.truncated)
          })
          .catch(() => {
            void window.api.saveAppState({ lastFolder: null })
          })
      }

      if (state.sessionTabs?.length) {
        const activePath = state.sessionActiveTabPath
        const orderedTabs = activePath
          ? [
              ...state.sessionTabs.filter((t) => t.path === activePath),
              ...state.sessionTabs.filter((t) => t.path !== activePath),
            ]
          : state.sessionTabs

        for (const tab of orderedTabs) {
          try {
            const content = await window.api.readFile(tab.path)
            openTab({ path: tab.path, content })
          } catch {
            // Skip unreadable session tabs
          }
        }

        if (state.sessionActiveTabPath) {
          const tabs = useAppStore.getState().tabs
          const active = tabs.find((t) => t.path === state.sessionActiveTabPath)
          if (active) {
            useAppStore.setState({ activeTabId: active.id })
          }
        }
      }

      useAppStore.setState({ initialized: true })
    })
  }, [setOpenFolder, openTab])
}
```

- [ ] **Step 4.2: Verify full verification check**
Ensure that formatting, linting, typechecking, and tests all still pass cleanly.
Run: `pnpm run typecheck && pnpm run lint && pnpm run fmt:check && pnpm run test`
Expected: PASS

---

### Task 5: Add CLI Script and Package Settings

Create the CLI launcher script at `bin/mdow` and ensure it resolves paths robustly.

**Files:**
- Create: `bin/mdow`

- [ ] **Step 5.1: Create directory bin if not exists**
Verify directory existence and write `bin/mdow` with correct path resolution logic.

```bash
#!/bin/sh

# Default to current directory if no args provided
if [ $# -eq 0 ]; then
  set -- "."
fi

ARGS=""
for arg in "$@"; do
  # Check if argument is a path that exists
  if [ -e "$arg" ]; then
    # Resolve relative paths to absolute paths
    ABS_DIR=$(cd "$(dirname "$arg")" && pwd)
    BASE=$(basename "$arg")
    if [ "$BASE" = "." ] || [ "$BASE" = "/" ]; then
      ABS_PATH="$ABS_DIR"
    else
      ABS_PATH="$ABS_DIR/$BASE"
    fi
    ARGS="$ARGS \"$ABS_PATH\""
  else
    ARGS="$ARGS \"$arg\""
  fi
done

# Detect if we are running in the development workspace repository
# by checking if pnpm-workspace.yaml exists in current or parent folders
IS_DEV=false
DIR=$(pwd)
while [ "$DIR" != "" ] && [ "$DIR" != "/" ]; do
  if [ -f "$DIR/pnpm-workspace.yaml" ]; then
    IS_DEV=true
    DEV_DIR="$DIR"
    break
  fi
  DIR=$(dirname "$DIR")
done

if [ "$IS_DEV" = true ]; then
  # Development: execute via pnpm dev in workspace
  eval pnpm --dir "$DEV_DIR" run --filter desktop dev -- $ARGS
else
  # Production: launch packaged app depending on OS
  if [ "$(uname)" = "Darwin" ]; then
    eval open -a "Mdow" --args $ARGS
  elif [ "$(expr substr $(uname -s) 1|5)" = "Linux" ]; then
    eval mdow $ARGS &
  else
    eval mdow $ARGS
  fi
fi
```

- [ ] **Step 5.2: Make the script executable**
Run: `chmod +x bin/mdow`
Expected: Success

- [ ] **Step 5.3: Verify CLI script manually**
Launch Mdow by calling `bin/mdow .` from the root of the project to test directory loading. Then run `bin/mdow README.md` to test file tab opening.

- [ ] **Step 5.4: Verify full project verification check**
Run: `pnpm run typecheck && pnpm run lint && pnpm run fmt:check && pnpm run test`
Expected: PASS

- [ ] **Step 5.5: Commit implementation**
Stage and commit all changes.
Run: `git add . && git commit -m "feat(desktop): implement multi-window single-instance CLI support"`
