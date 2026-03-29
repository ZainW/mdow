# mdview-electron Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a cross-platform Electron markdown viewer with the same rendering quality as the native macOS mdview app — md4x parsing, shiki syntax highlighting, mermaid diagrams, unified sidebar, command palette.

**Architecture:** electron-vite with three processes (main, preload, renderer). React 19 renderer with Zustand for app state, TanStack Query for async IPC operations, TanStack Virtual for sidebar list virtualization. Native OS title bars on all platforms.

**Tech Stack:** electron-vite 5, Electron 39+, React 19, TypeScript, Zustand, TanStack Query, TanStack Virtual, md4x, shiki, mermaid, chokidar, electron-store, Inter + Geist Mono fonts.

**Spec:** `docs/superpowers/specs/2026-03-29-mdview-electron-design.md`

---

## File Map

### Main Process (`src/main/`)

| File                         | Responsibility                                           |
| ---------------------------- | -------------------------------------------------------- |
| `src/main/index.ts`          | App lifecycle, window creation, window state restore     |
| `src/main/menu.ts`           | Native application menu (File > Open, Open Folder, etc.) |
| `src/main/ipc.ts`            | Register all ipcMain.handle and push-event wiring        |
| `src/main/file-service.ts`   | Read files, open file dialog, file watching              |
| `src/main/folder-service.ts` | Scan folder tree, open folder dialog, folder watching    |
| `src/main/store.ts`          | electron-store schema and accessor functions             |

### Preload (`src/preload/`)

| File                     | Responsibility                            |
| ------------------------ | ----------------------------------------- |
| `src/preload/index.ts`   | contextBridge exposing typed `window.api` |
| `src/preload/index.d.ts` | Global type declarations for `window.api` |

### Renderer (`src/renderer/`)

| File                                             | Responsibility                                                   |
| ------------------------------------------------ | ---------------------------------------------------------------- |
| `src/renderer/index.html`                        | HTML shell                                                       |
| `src/renderer/src/main.tsx`                      | React entry, QueryClientProvider                                 |
| `src/renderer/src/App.tsx`                       | Root layout: sidebar + content area                              |
| `src/renderer/src/components/Sidebar.tsx`        | Sidebar container: quick open bar, recents, folder tree, footer  |
| `src/renderer/src/components/RecentsList.tsx`    | Recent files list with TanStack Virtual                          |
| `src/renderer/src/components/FolderTree.tsx`     | Hierarchical folder tree with expand/collapse                    |
| `src/renderer/src/components/MarkdownView.tsx`   | Rendered markdown output area                                    |
| `src/renderer/src/components/CommandPalette.tsx` | Modal fuzzy search overlay                                       |
| `src/renderer/src/components/WelcomeView.tsx`    | Empty state with drag-drop and open buttons                      |
| `src/renderer/src/hooks/useFileContent.ts`       | TanStack Query hook wrapping file:read IPC                       |
| `src/renderer/src/hooks/useFolderTree.ts`        | TanStack Query hook wrapping folder:read-tree IPC                |
| `src/renderer/src/hooks/useRecents.ts`           | TanStack Query hook wrapping store:get-recents IPC               |
| `src/renderer/src/hooks/useTheme.ts`             | Listen for theme:changed, sync CSS class                         |
| `src/renderer/src/hooks/useIpcEvent.ts`          | Generic hook for subscribing to main-to-renderer events          |
| `src/renderer/src/store/app-store.ts`            | Zustand store: activeFile, sidebarWidth, sidebarOpen, openFolder |
| `src/renderer/src/lib/markdown.ts`               | md4x + shiki init, renderMarkdown function                       |
| `src/renderer/src/lib/mermaid.ts`                | Mermaid init + render helper                                     |
| `src/renderer/src/lib/fuzzy-search.ts`           | Fuzzy matching with scoring                                      |
| `src/renderer/src/lib/path-utils.ts`             | basename/shortenPath helpers for browser context                 |
| `src/renderer/src/assets/fonts/`                 | Inter Variable, Geist Mono Variable (woff2)                      |
| `src/renderer/src/assets/styles/index.css`       | Global styles, markdown CSS, theme variables                     |

---

## Task 1: Scaffold Project with electron-vite

**Files:**

- Create: `package.json`, `electron.vite.config.ts`, `tsconfig.json`, `tsconfig.node.json`, `tsconfig.web.json`
- Create: `src/main/index.ts`, `src/preload/index.ts`, `src/renderer/index.html`, `src/renderer/src/main.tsx`, `src/renderer/src/App.tsx`, `src/renderer/src/env.d.ts`

- [ ] **Step 1: Scaffold the project using electron-vite's React+TS template**

```bash
cd /Users/zain/labs/mdview-electron
npm create @quick-start/electron@latest . -- --template react-ts
```

If the tool doesn't support `.` as destination, scaffold to a temp name and move files. Answer prompts: project name `mdview-electron`, select React + TypeScript.

- [ ] **Step 2: Install dependencies**

```bash
cd /Users/zain/labs/mdview-electron
npm install
```

- [ ] **Step 3: Install project-specific dependencies**

```bash
npm install zustand @tanstack/react-query @tanstack/react-virtual electron-store chokidar md4x shiki mermaid
npm install -D vite-plugin-wasm vite-plugin-top-level-await
```

- [ ] **Step 4: Update `electron.vite.config.ts` with WASM support and aliases**

```typescript
import { resolve } from 'path'
import { defineConfig } from 'electron-vite'
import react from '@vitejs/plugin-react'
import wasm from 'vite-plugin-wasm'
import topLevelAwait from 'vite-plugin-top-level-await'

export default defineConfig({
  main: {},
  preload: {},
  renderer: {
    resolve: {
      alias: {
        '@renderer': resolve('src/renderer/src'),
      },
    },
    plugins: [react(), wasm(), topLevelAwait()],
    optimizeDeps: {
      exclude: ['md4x'],
    },
    assetsInclude: ['**/*.wasm'],
  },
})
```

- [ ] **Step 5: Verify dev server starts**

```bash
npm run dev
```

Expected: Electron window opens with the default template content. Close the window.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: scaffold electron-vite project with React+TS template and deps"
```

---

## Task 2: Fonts and Global Styles

**Files:**

- Create: `src/renderer/src/assets/fonts/InterVariable.woff2`
- Create: `src/renderer/src/assets/fonts/GeistMono-Variable.woff2`
- Create: `src/renderer/src/assets/styles/index.css`

- [ ] **Step 1: Copy font files from the original mdview project**

```bash
cp /Users/zain/labs/mdview/web/public/InterVariable.woff2 src/renderer/src/assets/fonts/
cp /Users/zain/labs/mdview/web/public/GeistMono-Variable.woff2 src/renderer/src/assets/fonts/
```

- [ ] **Step 2: Create `src/renderer/src/assets/styles/index.css`**

Port the CSS from the original `web/index.html`, adapted for the Electron app layout (sidebar + content):

```css
@font-face {
  font-family: 'Inter';
  src: url('../fonts/InterVariable.woff2') format('woff2');
  font-weight: 100 900;
  font-style: normal;
  font-display: block;
}

@font-face {
  font-family: 'Geist Mono';
  src: url('../fonts/GeistMono-Variable.woff2') format('woff2');
  font-weight: 100 900;
  font-style: normal;
  font-display: block;
}

:root {
  --bg: #ffffff;
  --text: #1d1d1f;
  --text-secondary: #6e6e73;
  --border: #d2d2d7;
  --code-bg: #f5f5f7;
  --blockquote-border: #d2d2d7;
  --blockquote-bg: #f5f5f7;
  --table-border: #d2d2d7;
  --table-stripe: #f5f5f7;
  --link: #0066cc;
  --sidebar-bg: #f5f5f7;
  --sidebar-hover: #e8e8ed;
  --sidebar-active: #0066cc;
  --accent: #0066cc;
}

html.dark {
  --bg: #1d1d1f;
  --text: #f5f5f7;
  --text-secondary: #a1a1a6;
  --border: #424245;
  --code-bg: #2c2c2e;
  --blockquote-border: #424245;
  --blockquote-bg: #2c2c2e;
  --table-border: #424245;
  --table-stripe: #2c2c2e;
  --link: #2997ff;
  --sidebar-bg: #161617;
  --sidebar-hover: #2c2c2e;
  --sidebar-active: #2997ff;
  --accent: #2997ff;
}

* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

html,
body,
#root {
  height: 100%;
  overflow: hidden;
}

body {
  font-family:
    'Inter',
    -apple-system,
    BlinkMacSystemFont,
    'Helvetica Neue',
    sans-serif;
  font-size: 14px;
  line-height: 1.6;
  color: var(--text);
  background: var(--bg);
  -webkit-font-smoothing: antialiased;
}

/* ===== App Layout ===== */

.app-layout {
  display: flex;
  height: 100%;
}

/* ===== Sidebar ===== */

.sidebar {
  width: 260px;
  min-width: 200px;
  max-width: 400px;
  background: var(--sidebar-bg);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  user-select: none;
}

.sidebar-header {
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
}

.quick-open-trigger {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border-radius: 6px;
  background: var(--bg);
  border: 1px solid var(--border);
  color: var(--text-secondary);
  font-size: 12px;
  cursor: pointer;
  width: 100%;
}

.quick-open-trigger:hover {
  border-color: var(--text-secondary);
}

.quick-open-trigger .shortcut {
  margin-left: auto;
  opacity: 0.4;
  font-size: 11px;
}

.sidebar-section-label {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-secondary);
  padding: 12px 16px 4px;
}

.sidebar-divider {
  margin: 8px 16px;
  border-top: 1px solid var(--border);
}

.sidebar-list {
  flex: 1;
  overflow-y: auto;
  padding: 0 8px;
}

.sidebar-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border-radius: 6px;
  font-size: 12px;
  color: var(--text);
  cursor: pointer;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.sidebar-item:hover {
  background: var(--sidebar-hover);
}

.sidebar-item.active {
  background: var(--sidebar-active);
  color: white;
}

.sidebar-item.indent-1 {
  padding-left: 28px;
}
.sidebar-item.indent-2 {
  padding-left: 44px;
}
.sidebar-item.indent-3 {
  padding-left: 60px;
}

.sidebar-footer {
  padding: 8px 16px;
  border-top: 1px solid var(--border);
  display: flex;
  gap: 8px;
}

.sidebar-footer button {
  flex: 1;
  padding: 4px 8px;
  font-size: 11px;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: var(--bg);
  color: var(--text);
  cursor: pointer;
}

.sidebar-footer button:hover {
  border-color: var(--text-secondary);
}

/* ===== Resize Handle ===== */

.resize-handle {
  width: 4px;
  cursor: col-resize;
  background: transparent;
  flex-shrink: 0;
}

.resize-handle:hover,
.resize-handle.active {
  background: var(--accent);
}

/* ===== Content Area ===== */

.content-area {
  flex: 1;
  overflow-y: auto;
  padding: 32px 48px;
}

/* ===== Markdown Styles ===== */

.markdown-body h1,
.markdown-body h2,
.markdown-body h3,
.markdown-body h4,
.markdown-body h5,
.markdown-body h6 {
  font-weight: 600;
  line-height: 1.3;
  margin-top: 1.5em;
  margin-bottom: 0.5em;
}

.markdown-body h1 {
  font-size: 2em;
  border-bottom: 1px solid var(--border);
  padding-bottom: 0.3em;
}
.markdown-body h2 {
  font-size: 1.5em;
  border-bottom: 1px solid var(--border);
  padding-bottom: 0.25em;
}
.markdown-body h3 {
  font-size: 1.25em;
}

.markdown-body a {
  color: var(--link);
  text-decoration: none;
}
.markdown-body a:hover {
  text-decoration: underline;
}

.markdown-body p {
  margin: 0.8em 0;
  font-size: 16px;
}
.markdown-body img {
  max-width: 100%;
  border-radius: 8px;
}
.markdown-body hr {
  border: none;
  border-top: 1px solid var(--border);
  margin: 2em 0;
}

.markdown-body code {
  font-family: 'Geist Mono', 'SF Mono', SFMono-Regular, Menlo, monospace;
  font-size: 0.875em;
  background: var(--code-bg);
  padding: 0.2em 0.4em;
  border-radius: 4px;
}

.markdown-body pre {
  background: var(--code-bg);
  border-radius: 8px;
  padding: 16px;
  overflow-x: auto;
  line-height: 1.45;
}

.markdown-body pre code {
  background: none;
  padding: 0;
  font-size: 0.85em;
}

.markdown-body .shiki {
  background: var(--code-bg) !important;
  border-radius: 8px;
  padding: 16px;
  overflow-x: auto;
}

html.dark .shiki,
html.dark .shiki span {
  color: var(--shiki-dark) !important;
  background-color: var(--shiki-dark-bg) !important;
}

.markdown-body blockquote {
  margin: 1em 0;
  padding: 0.5em 1em;
  border-left: 4px solid var(--blockquote-border);
  background: var(--blockquote-bg);
  border-radius: 0 8px 8px 0;
  color: var(--text-secondary);
}

.markdown-body blockquote p {
  margin: 0.4em 0;
}

.markdown-body table {
  border-collapse: collapse;
  width: 100%;
  margin: 1em 0;
}
.markdown-body th,
.markdown-body td {
  border: 1px solid var(--table-border);
  padding: 8px 12px;
  text-align: left;
}
.markdown-body th {
  font-weight: 600;
  background: var(--table-stripe);
}
.markdown-body tr:nth-child(even) {
  background: var(--table-stripe);
}

.markdown-body ul,
.markdown-body ol {
  padding-left: 2em;
}
.markdown-body li {
  margin: 0.25em 0;
}

.markdown-body .task-list-item {
  list-style: none;
  margin-left: -1.5em;
}
.markdown-body .task-list-item input {
  margin-right: 0.5em;
}

.markdown-body .mermaid {
  text-align: center;
  margin: 1.5em 0;
  background: transparent;
}
.markdown-body .mermaid svg {
  max-width: 100%;
}

.markdown-body .mermaid-error {
  color: #ff3b30;
  background: var(--code-bg);
  border-radius: 8px;
  padding: 12px 16px;
  margin: 1em 0;
  font-family: 'Geist Mono', 'SF Mono', monospace;
  font-size: 0.85em;
  white-space: pre-wrap;
}

/* ===== Welcome View ===== */

.welcome-view {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-secondary);
  gap: 16px;
}

.welcome-view h2 {
  font-size: 20px;
  font-weight: 600;
  color: var(--text);
}

.welcome-view p {
  font-size: 14px;
}

.welcome-view .drop-hint {
  border: 2px dashed var(--border);
  border-radius: 12px;
  padding: 32px 48px;
  text-align: center;
}

.welcome-view .drop-hint.drag-over {
  border-color: var(--accent);
  background: rgba(0, 102, 204, 0.05);
}

html.dark .welcome-view .drop-hint.drag-over {
  background: rgba(41, 151, 255, 0.05);
}

.welcome-view button {
  padding: 8px 20px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg);
  color: var(--text);
  font-size: 14px;
  cursor: pointer;
}

.welcome-view button:hover {
  border-color: var(--accent);
  color: var(--accent);
}

/* ===== Command Palette ===== */

.command-palette-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.3);
  display: flex;
  justify-content: center;
  padding-top: 20vh;
  z-index: 100;
}

.command-palette {
  width: 500px;
  max-height: 400px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 12px;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.2);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.command-palette-input {
  padding: 12px 16px;
  border: none;
  border-bottom: 1px solid var(--border);
  background: transparent;
  color: var(--text);
  font-size: 15px;
  font-family: inherit;
  outline: none;
  width: 100%;
}

.command-palette-results {
  flex: 1;
  overflow-y: auto;
  padding: 4px;
}

.command-palette-item {
  display: flex;
  flex-direction: column;
  padding: 8px 12px;
  border-radius: 8px;
  cursor: pointer;
  gap: 2px;
}

.command-palette-item:hover,
.command-palette-item.selected {
  background: var(--sidebar-hover);
}

.command-palette-item .filename {
  font-size: 14px;
  color: var(--text);
}

.command-palette-item .filepath {
  font-size: 12px;
  color: var(--text-secondary);
}

.command-palette-empty {
  padding: 16px;
  text-align: center;
  color: var(--text-secondary);
  font-size: 13px;
}
```

- [ ] **Step 3: Import the stylesheet in `src/renderer/src/main.tsx`**

Replace the default template content of `src/renderer/src/main.tsx` with:

```tsx
import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import './assets/styles/index.css'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
```

- [ ] **Step 4: Create a minimal `src/renderer/src/App.tsx`**

```tsx
function App(): React.JSX.Element {
  return (
    <div className="app-layout">
      <div className="sidebar">
        <div className="sidebar-header">
          <div className="quick-open-trigger">
            <span>Quick Open</span>
            <span className="shortcut">&#x2318;K</span>
          </div>
        </div>
      </div>
      <div className="content-area">
        <p>mdview-electron</p>
      </div>
    </div>
  )
}

export default App
```

- [ ] **Step 5: Verify the app renders with the correct fonts and layout**

```bash
npm run dev
```

Expected: Electron window with a sidebar on the left (light gray background, quick open bar) and content area on the right. Text uses Inter font.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: add fonts, global CSS with light/dark theme vars, and app shell layout"
```

---

## Task 3: Preload Script and Typed API

**Files:**

- Modify: `src/preload/index.ts`
- Create: `src/preload/index.d.ts`

- [ ] **Step 1: Define the API interface and implement the preload bridge**

Replace `src/preload/index.ts` with:

```typescript
import { contextBridge, ipcRenderer } from 'electron'

export interface FileResult {
  path: string
  content: string
}

export interface TreeNode {
  name: string
  path: string
  isDirectory: boolean
  children?: TreeNode[]
}

export interface AppState {
  sidebarWidth: number
  lastFolder: string | null
  windowBounds: { x: number; y: number; width: number; height: number } | null
}

export interface ElectronAPI {
  // Renderer to Main (invoke)
  openFileDialog: () => Promise<FileResult | null>
  readFile: (path: string) => Promise<string>
  openFolderDialog: () => Promise<{ path: string; tree: TreeNode[] } | null>
  readFolderTree: (folderPath: string) => Promise<TreeNode[]>
  getRecents: () => Promise<string[]>
  getAppState: () => Promise<AppState>
  saveAppState: (state: Partial<AppState>) => Promise<void>
  addRecent: (filePath: string) => Promise<void>
  showInFolder: (filePath: string) => Promise<void>

  // Main to Renderer (on/off)
  onFileChanged: (callback: (content: string) => void) => () => void
  onFolderChanged: (callback: (tree: TreeNode[]) => void) => () => void
  onThemeChanged: (callback: (isDark: boolean) => void) => () => void
  onMenuOpenFile: (callback: () => void) => () => void
  onMenuOpenFolder: (callback: () => void) => () => void
  onFileOpened: (callback: (file: FileResult) => void) => () => void
}

const api: ElectronAPI = {
  openFileDialog: () => ipcRenderer.invoke('file:open-dialog'),
  readFile: (path) => ipcRenderer.invoke('file:read', path),
  openFolderDialog: () => ipcRenderer.invoke('folder:open-dialog'),
  readFolderTree: (folderPath) => ipcRenderer.invoke('folder:read-tree', folderPath),
  getRecents: () => ipcRenderer.invoke('store:get-recents'),
  getAppState: () => ipcRenderer.invoke('store:get-state'),
  saveAppState: (state) => ipcRenderer.invoke('store:save-state', state),
  addRecent: (filePath) => ipcRenderer.invoke('store:add-recent', filePath),
  showInFolder: (filePath) => ipcRenderer.invoke('shell:show-in-folder', filePath),

  onFileChanged: (callback) => {
    const handler = (_: Electron.IpcRendererEvent, content: string) => callback(content)
    ipcRenderer.on('file:changed', handler)
    return () => ipcRenderer.removeListener('file:changed', handler)
  },
  onFolderChanged: (callback) => {
    const handler = (_: Electron.IpcRendererEvent, tree: TreeNode[]) => callback(tree)
    ipcRenderer.on('folder:changed', handler)
    return () => ipcRenderer.removeListener('folder:changed', handler)
  },
  onThemeChanged: (callback) => {
    const handler = (_: Electron.IpcRendererEvent, isDark: boolean) => callback(isDark)
    ipcRenderer.on('theme:changed', handler)
    return () => ipcRenderer.removeListener('theme:changed', handler)
  },
  onMenuOpenFile: (callback) => {
    const handler = () => callback()
    ipcRenderer.on('menu:open-file', handler)
    return () => ipcRenderer.removeListener('menu:open-file', handler)
  },
  onMenuOpenFolder: (callback) => {
    const handler = () => callback()
    ipcRenderer.on('menu:open-folder', handler)
    return () => ipcRenderer.removeListener('menu:open-folder', handler)
  },
  onFileOpened: (callback) => {
    const handler = (_: Electron.IpcRendererEvent, file: FileResult) => callback(file)
    ipcRenderer.on('file:opened', handler)
    return () => ipcRenderer.removeListener('file:opened', handler)
  },
}

contextBridge.exposeInMainWorld('api', api)
```

- [ ] **Step 2: Create `src/preload/index.d.ts`**

```typescript
import type { ElectronAPI } from './index'

declare global {
  interface Window {
    api: ElectronAPI
  }
}
```

- [ ] **Step 3: Ensure `tsconfig.web.json` includes the preload types**

Check that `src/preload/*.d.ts` is in the `include` array of `tsconfig.web.json`. The template should already include it. If not, add it:

```json
{
  "include": ["src/renderer/src/env.d.ts", "src/renderer/src/**/*", "src/preload/*.d.ts"]
}
```

- [ ] **Step 4: Verify TypeScript compiles**

```bash
npx tsc --noEmit
```

Expected: No errors.

- [ ] **Step 5: Commit**

```bash
git add src/preload/
git commit -m "feat: add typed preload bridge with all IPC channels"
```

---

## Task 4: electron-store and Persistence

**Files:**

- Create: `src/main/store.ts`

- [ ] **Step 1: Create `src/main/store.ts`**

```typescript
import Store from 'electron-store'

interface StoreSchema {
  recents: string[]
  lastFolder: string | null
  sidebarWidth: number
  windowBounds: { x: number; y: number; width: number; height: number } | null
}

const store = new Store<StoreSchema>({
  defaults: {
    recents: [],
    lastFolder: null,
    sidebarWidth: 260,
    windowBounds: null,
  },
})

const MAX_RECENTS = 20

export function getRecents(): string[] {
  return store.get('recents')
}

export function addRecent(filePath: string): void {
  const recents = store.get('recents').filter((r) => r !== filePath)
  recents.unshift(filePath)
  store.set('recents', recents.slice(0, MAX_RECENTS))
}

export function getAppState() {
  return {
    sidebarWidth: store.get('sidebarWidth'),
    lastFolder: store.get('lastFolder'),
    windowBounds: store.get('windowBounds'),
  }
}

export function saveAppState(state: Partial<StoreSchema>): void {
  for (const [key, value] of Object.entries(state)) {
    store.set(key as keyof StoreSchema, value)
  }
}

export function getWindowBounds() {
  return store.get('windowBounds')
}

export function saveWindowBounds(bounds: { x: number; y: number; width: number; height: number }) {
  store.set('windowBounds', bounds)
}

export function getLastFolder(): string | null {
  return store.get('lastFolder')
}

export function setLastFolder(folder: string | null): void {
  store.set('lastFolder', folder)
}
```

- [ ] **Step 2: Commit**

```bash
git add src/main/store.ts
git commit -m "feat: add electron-store with recents, window bounds, and app state"
```

---

## Task 5: File Service

**Files:**

- Create: `src/main/file-service.ts`

- [ ] **Step 1: Create `src/main/file-service.ts`**

```typescript
import { dialog, BrowserWindow } from 'electron'
import { readFile } from 'fs/promises'
import { watch, type FSWatcher } from 'chokidar'

let fileWatcher: FSWatcher | null = null

export async function openFileDialog(win: BrowserWindow) {
  const result = await dialog.showOpenDialog(win, {
    properties: ['openFile'],
    filters: [{ name: 'Markdown', extensions: ['md', 'markdown', 'mdx'] }],
  })

  if (result.canceled || result.filePaths.length === 0) return null

  const path = result.filePaths[0]
  const content = await readFile(path, 'utf-8')
  return { path, content }
}

export async function readFileContent(path: string): Promise<string> {
  return readFile(path, 'utf-8')
}

export function watchFile(filePath: string, onChange: (content: string) => void): void {
  unwatchFile()
  fileWatcher = watch(filePath, { ignoreInitial: true })
  let debounceTimer: ReturnType<typeof setTimeout> | null = null

  fileWatcher.on('change', async () => {
    if (debounceTimer) clearTimeout(debounceTimer)
    debounceTimer = setTimeout(async () => {
      try {
        const content = await readFile(filePath, 'utf-8')
        onChange(content)
      } catch {
        // File might be temporarily unavailable during save
      }
    }, 300)
  })
}

export function unwatchFile(): void {
  if (fileWatcher) {
    fileWatcher.close()
    fileWatcher = null
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/main/file-service.ts
git commit -m "feat: add file service with dialog, read, and chokidar watching"
```

---

## Task 6: Folder Service

**Files:**

- Create: `src/main/folder-service.ts`

- [ ] **Step 1: Create `src/main/folder-service.ts`**

```typescript
import { dialog, BrowserWindow } from 'electron'
import { readdir } from 'fs/promises'
import { join } from 'path'
import { watch, type FSWatcher } from 'chokidar'

export interface TreeNode {
  name: string
  path: string
  isDirectory: boolean
  children?: TreeNode[]
}

let folderWatcher: FSWatcher | null = null

const MD_EXTENSIONS = new Set(['.md', '.markdown', '.mdx'])

function isMdFile(name: string): boolean {
  const dotIndex = name.lastIndexOf('.')
  if (dotIndex === -1) return false
  const ext = name.substring(dotIndex).toLowerCase()
  return MD_EXTENSIONS.has(ext)
}

export async function scanFolder(folderPath: string): Promise<TreeNode[]> {
  const entries = await readdir(folderPath, { withFileTypes: true })
  const nodes: TreeNode[] = []

  const sorted = entries.sort((a, b) => {
    if (a.isDirectory() && !b.isDirectory()) return -1
    if (!a.isDirectory() && b.isDirectory()) return 1
    return a.name.localeCompare(b.name)
  })

  for (const entry of sorted) {
    if (entry.name.startsWith('.')) continue

    const fullPath = join(folderPath, entry.name)

    if (entry.isDirectory()) {
      const children = await scanFolder(fullPath)
      // Only include directories that transitively contain .md files
      if (children.length > 0) {
        nodes.push({ name: entry.name, path: fullPath, isDirectory: true, children })
      }
    } else if (isMdFile(entry.name)) {
      nodes.push({ name: entry.name, path: fullPath, isDirectory: false })
    }
  }

  return nodes
}

export async function openFolderDialog(win: BrowserWindow) {
  const result = await dialog.showOpenDialog(win, {
    properties: ['openDirectory'],
  })

  if (result.canceled || result.filePaths.length === 0) return null

  const folderPath = result.filePaths[0]
  const tree = await scanFolder(folderPath)
  return { path: folderPath, tree }
}

export function watchFolder(folderPath: string, onChange: (tree: TreeNode[]) => void): void {
  unwatchFolder()

  folderWatcher = watch(folderPath, {
    ignoreInitial: true,
    ignored: /(^|[/\\])\./,
    depth: 10,
  })

  let debounceTimer: ReturnType<typeof setTimeout> | null = null

  const handleChange = () => {
    if (debounceTimer) clearTimeout(debounceTimer)
    debounceTimer = setTimeout(async () => {
      try {
        const tree = await scanFolder(folderPath)
        onChange(tree)
      } catch {
        // Folder might have been deleted
      }
    }, 1000)
  }

  folderWatcher.on('add', handleChange)
  folderWatcher.on('unlink', handleChange)
  folderWatcher.on('addDir', handleChange)
  folderWatcher.on('unlinkDir', handleChange)
}

export function unwatchFolder(): void {
  if (folderWatcher) {
    folderWatcher.close()
    folderWatcher = null
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/main/folder-service.ts
git commit -m "feat: add folder service with recursive scan, md filtering, and watching"
```

---

## Task 7: IPC Handler Registration

**Files:**

- Create: `src/main/ipc.ts`

- [ ] **Step 1: Create `src/main/ipc.ts`**

```typescript
import { ipcMain, shell, BrowserWindow } from 'electron'
import { openFileDialog, readFileContent, watchFile } from './file-service'
import { openFolderDialog, scanFolder, watchFolder } from './folder-service'
import { getRecents, addRecent, getAppState, saveAppState, setLastFolder } from './store'

export function registerIpcHandlers(getMainWindow: () => BrowserWindow | null): void {
  ipcMain.handle('file:open-dialog', async () => {
    const win = getMainWindow()
    if (!win) return null
    const result = await openFileDialog(win)
    if (result) {
      addRecent(result.path)
      watchFile(result.path, (content) => {
        win.webContents.send('file:changed', content)
      })
    }
    return result
  })

  ipcMain.handle('file:read', async (_, path: string) => {
    const content = await readFileContent(path)
    const win = getMainWindow()
    if (win) {
      addRecent(path)
      watchFile(path, (newContent) => {
        win.webContents.send('file:changed', newContent)
      })
    }
    return content
  })

  ipcMain.handle('folder:open-dialog', async () => {
    const win = getMainWindow()
    if (!win) return null
    const result = await openFolderDialog(win)
    if (result) {
      setLastFolder(result.path)
      watchFolder(result.path, (tree) => {
        win.webContents.send('folder:changed', tree)
      })
    }
    return result
  })

  ipcMain.handle('folder:read-tree', async (_, folderPath: string) => {
    const win = getMainWindow()
    const tree = await scanFolder(folderPath)
    if (win) {
      setLastFolder(folderPath)
      watchFolder(folderPath, (newTree) => {
        win.webContents.send('folder:changed', newTree)
      })
    }
    return tree
  })

  ipcMain.handle('store:get-recents', () => getRecents())
  ipcMain.handle('store:get-state', () => getAppState())
  ipcMain.handle('store:save-state', (_, state) => saveAppState(state))
  ipcMain.handle('store:add-recent', (_, filePath: string) => addRecent(filePath))

  ipcMain.handle('shell:show-in-folder', (_, filePath: string) => {
    shell.showItemInFolder(filePath)
  })
}
```

- [ ] **Step 2: Commit**

```bash
git add src/main/ipc.ts
git commit -m "feat: register all IPC handlers with file/folder watching wiring"
```

---

## Task 8: Native Menu

**Files:**

- Create: `src/main/menu.ts`

- [ ] **Step 1: Create `src/main/menu.ts`**

```typescript
import { Menu, app, BrowserWindow } from 'electron'

export function createMenu(getMainWindow: () => BrowserWindow | null): void {
  const isMac = process.platform === 'darwin'

  const template: Electron.MenuItemConstructorOptions[] = [
    ...(isMac
      ? [
          {
            label: app.name,
            submenu: [
              { role: 'about' as const },
              { type: 'separator' as const },
              { role: 'services' as const },
              { type: 'separator' as const },
              { role: 'hide' as const },
              { role: 'hideOthers' as const },
              { role: 'unhide' as const },
              { type: 'separator' as const },
              { role: 'quit' as const },
            ],
          },
        ]
      : []),
    {
      label: 'File',
      submenu: [
        {
          label: 'Open File...',
          accelerator: 'CmdOrCtrl+O',
          click: () => getMainWindow()?.webContents.send('menu:open-file'),
        },
        {
          label: 'Open Folder...',
          accelerator: 'CmdOrCtrl+Shift+O',
          click: () => getMainWindow()?.webContents.send('menu:open-folder'),
        },
        { type: 'separator' },
        isMac ? { role: 'close' } : { role: 'quit' },
      ],
    },
    {
      label: 'Edit',
      submenu: [
        { role: 'undo' },
        { role: 'redo' },
        { type: 'separator' },
        { role: 'cut' },
        { role: 'copy' },
        { role: 'paste' },
        { role: 'selectAll' },
      ],
    },
    {
      label: 'View',
      submenu: [
        {
          label: 'Toggle Sidebar',
          accelerator: 'CmdOrCtrl+B',
          click: () => getMainWindow()?.webContents.send('menu:toggle-sidebar'),
        },
        { type: 'separator' },
        { role: 'reload' },
        { role: 'forceReload' },
        { role: 'toggleDevTools' },
        { type: 'separator' },
        { role: 'resetZoom' },
        { role: 'zoomIn' },
        { role: 'zoomOut' },
        { type: 'separator' },
        { role: 'togglefullscreen' },
      ],
    },
    {
      label: 'Window',
      submenu: [
        { role: 'minimize' },
        { role: 'zoom' },
        ...(isMac
          ? [{ type: 'separator' as const }, { role: 'front' as const }]
          : [{ role: 'close' as const }]),
      ],
    },
  ]

  const menu = Menu.buildFromTemplate(template)
  Menu.setApplicationMenu(menu)
}
```

- [ ] **Step 2: Commit**

```bash
git add src/main/menu.ts
git commit -m "feat: add native menu with File, Edit, View, Window menus"
```

---

## Task 9: Main Process Entry Point

**Files:**

- Modify: `src/main/index.ts`

- [ ] **Step 1: Replace `src/main/index.ts` with full app lifecycle**

```typescript
import { app, BrowserWindow, nativeTheme } from 'electron'
import { join } from 'path'
import { is } from '@electron-toolkit/utils'
import { registerIpcHandlers } from './ipc'
import { createMenu } from './menu'
import { getWindowBounds, saveWindowBounds, getLastFolder } from './store'
import { scanFolder, watchFolder } from './folder-service'
import { readFileContent } from './file-service'

let mainWindow: BrowserWindow | null = null

function getMainWindow(): BrowserWindow | null {
  return mainWindow
}

function createWindow(): void {
  const savedBounds = getWindowBounds()

  mainWindow = new BrowserWindow({
    width: savedBounds?.width ?? 1000,
    height: savedBounds?.height ?? 700,
    x: savedBounds?.x,
    y: savedBounds?.y,
    minWidth: 600,
    minHeight: 400,
    show: false,
    webPreferences: {
      preload: join(__dirname, '../preload/index.js'),
      sandbox: false,
    },
  })

  mainWindow.on('ready-to-show', () => {
    mainWindow!.show()
  })

  // Save window bounds on move/resize
  const saveBounds = () => {
    if (mainWindow && !mainWindow.isDestroyed()) {
      saveWindowBounds(mainWindow.getBounds())
    }
  }
  mainWindow.on('resized', saveBounds)
  mainWindow.on('moved', saveBounds)

  // Theme change detection
  nativeTheme.on('updated', () => {
    mainWindow?.webContents.send('theme:changed', nativeTheme.shouldUseDarkColors)
  })

  // Send initial theme state and restore last folder once renderer is ready
  mainWindow.webContents.on('did-finish-load', async () => {
    mainWindow?.webContents.send('theme:changed', nativeTheme.shouldUseDarkColors)

    // Restore last folder
    const lastFolder = getLastFolder()
    if (lastFolder) {
      try {
        const tree = await scanFolder(lastFolder)
        mainWindow?.webContents.send('folder:changed', tree)
        watchFolder(lastFolder, (newTree) => {
          mainWindow?.webContents.send('folder:changed', newTree)
        })
      } catch {
        // Folder no longer exists
      }
    }

    // Handle CLI arguments (file passed as argument)
    const filePath = process.argv.find(
      (arg) => arg.endsWith('.md') || arg.endsWith('.markdown') || arg.endsWith('.mdx'),
    )
    if (filePath) {
      try {
        const content = await readFileContent(filePath)
        mainWindow?.webContents.send('file:opened', { path: filePath, content })
      } catch {
        // Invalid file path
      }
    }
  })

  // Load the renderer
  if (is.dev && process.env['ELECTRON_RENDERER_URL']) {
    mainWindow.loadURL(process.env['ELECTRON_RENDERER_URL'])
  } else {
    mainWindow.loadFile(join(__dirname, '../renderer/index.html'))
  }

  mainWindow.on('closed', () => {
    mainWindow = null
  })
}

app.whenReady().then(() => {
  registerIpcHandlers(getMainWindow)
  createMenu(getMainWindow)
  createWindow()

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit()
})

// Handle file open on macOS (double-click .md file)
app.on('open-file', async (event, path) => {
  event.preventDefault()
  if (mainWindow) {
    try {
      const content = await readFileContent(path)
      mainWindow.webContents.send('file:opened', { path, content })
    } catch {
      // Invalid file
    }
  }
})
```

- [ ] **Step 2: Verify the app starts**

```bash
npm run dev
```

Expected: Electron window opens with the sidebar + content layout. No errors in the terminal.

- [ ] **Step 3: Commit**

```bash
git add src/main/index.ts
git commit -m "feat: main process with window lifecycle, theme detection, and folder restore"
```

---

## Task 10: Zustand Store

**Files:**

- Create: `src/renderer/src/store/app-store.ts`

- [ ] **Step 1: Create `src/renderer/src/store/app-store.ts`**

```typescript
import { create } from 'zustand'

interface ActiveFile {
  path: string
  content: string
}

interface TreeNode {
  name: string
  path: string
  isDirectory: boolean
  children?: TreeNode[]
}

interface AppStore {
  // Active file
  activeFile: ActiveFile | null
  setActiveFile: (file: ActiveFile | null) => void
  updateActiveFileContent: (content: string) => void

  // Sidebar
  sidebarOpen: boolean
  sidebarWidth: number
  toggleSidebar: () => void
  setSidebarWidth: (width: number) => void

  // Folder
  openFolderPath: string | null
  folderTree: TreeNode[]
  setOpenFolder: (path: string, tree: TreeNode[]) => void
  setFolderTree: (tree: TreeNode[]) => void

  // Command palette
  commandPaletteOpen: boolean
  setCommandPaletteOpen: (open: boolean) => void
}

export const useAppStore = create<AppStore>((set) => ({
  activeFile: null,
  setActiveFile: (file) => set({ activeFile: file }),
  updateActiveFileContent: (content) =>
    set((state) => {
      if (!state.activeFile) return state
      return { activeFile: { ...state.activeFile, content } }
    }),

  sidebarOpen: true,
  sidebarWidth: 260,
  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
  setSidebarWidth: (width) => set({ sidebarWidth: width }),

  openFolderPath: null,
  folderTree: [],
  setOpenFolder: (path, tree) => set({ openFolderPath: path, folderTree: tree }),
  setFolderTree: (tree) => set({ folderTree: tree }),

  commandPaletteOpen: false,
  setCommandPaletteOpen: (open) => set({ commandPaletteOpen: open }),
}))
```

- [ ] **Step 2: Commit**

```bash
git add src/renderer/src/store/
git commit -m "feat: add Zustand store for app state"
```

---

## Task 11: IPC Event Hook and TanStack Query Hooks

**Files:**

- Create: `src/renderer/src/hooks/useIpcEvent.ts`
- Create: `src/renderer/src/hooks/useFileContent.ts`
- Create: `src/renderer/src/hooks/useFolderTree.ts`
- Create: `src/renderer/src/hooks/useRecents.ts`
- Create: `src/renderer/src/hooks/useTheme.ts`

- [ ] **Step 1: Create `src/renderer/src/hooks/useIpcEvent.ts`**

```typescript
import { useEffect } from 'react'

export function useIpcEvent<T>(
  subscribe: (callback: (data: T) => void) => () => void,
  handler: (data: T) => void,
): void {
  useEffect(() => {
    const unsubscribe = subscribe(handler)
    return unsubscribe
  }, [subscribe, handler])
}
```

- [ ] **Step 2: Create `src/renderer/src/hooks/useFileContent.ts`**

```typescript
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useCallback } from 'react'
import { useIpcEvent } from './useIpcEvent'
import { useAppStore } from '../store/app-store'

export function useFileContent(filePath: string | null) {
  const queryClient = useQueryClient()
  const updateActiveFileContent = useAppStore((s) => s.updateActiveFileContent)

  const handleFileChanged = useCallback(
    (content: string) => {
      if (filePath) {
        queryClient.setQueryData(['file', filePath], content)
        updateActiveFileContent(content)
      }
    },
    [filePath, queryClient, updateActiveFileContent],
  )

  useIpcEvent(window.api.onFileChanged, handleFileChanged)

  return useQuery({
    queryKey: ['file', filePath],
    queryFn: () => window.api.readFile(filePath!),
    enabled: !!filePath,
    staleTime: Infinity,
  })
}
```

- [ ] **Step 3: Create `src/renderer/src/hooks/useFolderTree.ts`**

```typescript
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useCallback } from 'react'
import { useIpcEvent } from './useIpcEvent'
import { useAppStore } from '../store/app-store'

export function useFolderTree(folderPath: string | null) {
  const queryClient = useQueryClient()
  const setFolderTree = useAppStore((s) => s.setFolderTree)

  const handleFolderChanged = useCallback(
    (tree: any[]) => {
      if (folderPath) {
        queryClient.setQueryData(['folder', folderPath], tree)
        setFolderTree(tree)
      }
    },
    [folderPath, queryClient, setFolderTree],
  )

  useIpcEvent(window.api.onFolderChanged, handleFolderChanged)

  return useQuery({
    queryKey: ['folder', folderPath],
    queryFn: () => window.api.readFolderTree(folderPath!),
    enabled: !!folderPath,
    staleTime: Infinity,
  })
}
```

- [ ] **Step 4: Create `src/renderer/src/hooks/useRecents.ts`**

```typescript
import { useQuery } from '@tanstack/react-query'

export function useRecents() {
  return useQuery({
    queryKey: ['recents'],
    queryFn: () => window.api.getRecents(),
  })
}
```

- [ ] **Step 5: Create `src/renderer/src/hooks/useTheme.ts`**

```typescript
import { useCallback, useEffect } from 'react'

export function useTheme(): void {
  const handleThemeChanged = useCallback((isDark: boolean) => {
    document.documentElement.classList.toggle('dark', isDark)
  }, [])

  useEffect(() => {
    const unsubscribe = window.api.onThemeChanged(handleThemeChanged)
    return unsubscribe
  }, [handleThemeChanged])
}
```

- [ ] **Step 6: Commit**

```bash
git add src/renderer/src/hooks/
git commit -m "feat: add TanStack Query hooks and IPC event hook"
```

---

## Task 12: Markdown Rendering Library

**Files:**

- Create: `src/renderer/src/lib/markdown.ts`
- Create: `src/renderer/src/lib/mermaid.ts`

- [ ] **Step 1: Create `src/renderer/src/lib/markdown.ts`**

Port the rendering pipeline from the original mdview `web/main.js`:

```typescript
import { createHighlighterCore } from 'shiki/core'
import { createJavaScriptRegexEngine } from 'shiki/engine/javascript'
import githubLight from 'shiki/themes/github-light.mjs'
import githubDark from 'shiki/themes/github-dark.mjs'

import langJavascript from 'shiki/langs/javascript.mjs'
import langTypescript from 'shiki/langs/typescript.mjs'
import langPython from 'shiki/langs/python.mjs'
import langRust from 'shiki/langs/rust.mjs'
import langGo from 'shiki/langs/go.mjs'
import langJava from 'shiki/langs/java.mjs'
import langC from 'shiki/langs/c.mjs'
import langCpp from 'shiki/langs/cpp.mjs'
import langCsharp from 'shiki/langs/csharp.mjs'
import langRuby from 'shiki/langs/ruby.mjs'
import langSwift from 'shiki/langs/swift.mjs'
import langKotlin from 'shiki/langs/kotlin.mjs'
import langHtml from 'shiki/langs/html.mjs'
import langCss from 'shiki/langs/css.mjs'
import langJson from 'shiki/langs/json.mjs'
import langYaml from 'shiki/langs/yaml.mjs'
import langToml from 'shiki/langs/toml.mjs'
import langXml from 'shiki/langs/xml.mjs'
import langMarkdown from 'shiki/langs/markdown.mjs'
import langSql from 'shiki/langs/sql.mjs'
import langBash from 'shiki/langs/bash.mjs'
import langShell from 'shiki/langs/shellscript.mjs'
import langDiff from 'shiki/langs/diff.mjs'
import langGraphql from 'shiki/langs/graphql.mjs'
import langDockerfile from 'shiki/langs/dockerfile.mjs'
import langLua from 'shiki/langs/lua.mjs'
import langZig from 'shiki/langs/zig.mjs'
import langElixir from 'shiki/langs/elixir.mjs'
import langHaskell from 'shiki/langs/haskell.mjs'
import langOcaml from 'shiki/langs/ocaml.mjs'
import langJsx from 'shiki/langs/jsx.mjs'
import langTsx from 'shiki/langs/tsx.mjs'
import langPhp from 'shiki/langs/php.mjs'

import { init as initMd4x, renderToHtml } from 'md4x/wasm'

const langs = [
  langJavascript,
  langTypescript,
  langPython,
  langRust,
  langGo,
  langJava,
  langC,
  langCpp,
  langCsharp,
  langRuby,
  langSwift,
  langKotlin,
  langHtml,
  langCss,
  langJson,
  langYaml,
  langToml,
  langXml,
  langMarkdown,
  langSql,
  langBash,
  langShell,
  langDiff,
  langGraphql,
  langDockerfile,
  langLua,
  langZig,
  langElixir,
  langHaskell,
  langOcaml,
  langJsx,
  langTsx,
  langPhp,
]

let highlighter: Awaited<ReturnType<typeof createHighlighterCore>> | null = null
let initialized = false

export async function initMarkdown(): Promise<void> {
  if (initialized) return

  await initMd4x()

  highlighter = await createHighlighterCore({
    themes: [githubLight, githubDark],
    langs,
    engine: createJavaScriptRegexEngine(),
  })

  initialized = true
}

function highlightCode(code: string, lang: string | null): string {
  if (!highlighter) {
    const escaped = code.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
    return `<pre><code>${escaped}</code></pre>`
  }

  const loaded = highlighter.getLoadedLanguages()

  if (lang && loaded.includes(lang)) {
    return highlighter.codeToHtml(code, {
      lang,
      themes: { light: 'github-light', dark: 'github-dark' },
    })
  }

  const escaped = code.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
  return `<pre><code>${escaped}</code></pre>`
}

export interface RenderResult {
  html: string
  mermaidBlocks: { id: string; code: string }[]
}

let mermaidCounter = 0

export function renderMarkdown(text: string): RenderResult {
  const rawHtml = renderToHtml(text)

  const wrapper = document.createElement('div')
  wrapper.innerHTML = rawHtml

  const mermaidBlocks: { id: string; code: string }[] = []
  mermaidCounter = 0

  const codeBlocks = wrapper.querySelectorAll('pre > code')
  for (const block of codeBlocks) {
    const langClass = [...block.classList].find((c) => c.startsWith('language-'))
    const lang = langClass ? langClass.replace('language-', '') : null
    const code = block.textContent || ''

    if (lang === 'mermaid') {
      const id = `mermaid-${mermaidCounter++}`
      const mermaidDiv = document.createElement('div')
      mermaidDiv.className = 'mermaid'
      mermaidDiv.id = id
      block.closest('pre')!.replaceWith(mermaidDiv)
      mermaidBlocks.push({ id, code })
    } else {
      const highlighted = highlightCode(code, lang)
      const temp = document.createElement('div')
      temp.innerHTML = highlighted
      if (temp.firstElementChild) {
        block.closest('pre')!.replaceWith(temp.firstElementChild)
      }
    }
  }

  return { html: wrapper.innerHTML, mermaidBlocks }
}
```

- [ ] **Step 2: Create `src/renderer/src/lib/mermaid.ts`**

```typescript
import mermaid from 'mermaid'

let mermaidInitialized = false

export function initMermaid(isDark: boolean): void {
  mermaid.initialize({
    startOnLoad: false,
    theme: isDark ? 'dark' : 'default',
    securityLevel: 'loose',
  })
  mermaidInitialized = true
}

export function updateMermaidTheme(isDark: boolean): void {
  mermaid.initialize({
    startOnLoad: false,
    theme: isDark ? 'dark' : 'default',
    securityLevel: 'loose',
  })
}

export async function renderMermaidBlocks(blocks: { id: string; code: string }[]): Promise<void> {
  if (!mermaidInitialized) return

  for (const block of blocks) {
    const el = document.getElementById(block.id)
    if (!el) continue

    try {
      const { svg } = await mermaid.render(`${block.id}-svg`, block.code)
      el.innerHTML = svg
    } catch (e) {
      el.className = 'mermaid-error'
      el.textContent = `Mermaid diagram error: ${e instanceof Error ? e.message : String(e)}`
      // Clean up orphaned SVG element that mermaid may have left
      const errorSvg = document.getElementById(`d${block.id}-svg`)
      if (errorSvg) errorSvg.remove()
    }
  }
}
```

- [ ] **Step 3: Commit**

```bash
git add src/renderer/src/lib/markdown.ts src/renderer/src/lib/mermaid.ts
git commit -m "feat: add markdown rendering pipeline (md4x + shiki + mermaid)"
```

---

## Task 13: Fuzzy Search and Path Utils

**Files:**

- Create: `src/renderer/src/lib/fuzzy-search.ts`
- Create: `src/renderer/src/lib/path-utils.ts`

- [ ] **Step 1: Create `src/renderer/src/lib/fuzzy-search.ts`**

Port the scoring algorithm from the original mdview:

```typescript
export interface SearchResult {
  path: string
  name: string
  score: number
}

function fuzzyScore(query: string, target: string): number {
  const q = query.toLowerCase()
  const t = target.toLowerCase()

  let score = 0
  let queryIndex = 0
  let consecutive = 0

  for (let i = 0; i < t.length && queryIndex < q.length; i++) {
    if (t[i] === q[queryIndex]) {
      score += 1

      // Bonus for consecutive matches
      if (consecutive > 0) {
        score += consecutive * 2
      }
      consecutive++

      // Bonus for matching at word boundary
      if (i === 0 || t[i - 1] === '/' || t[i - 1] === '-' || t[i - 1] === '_' || t[i - 1] === '.') {
        score += 5
      }

      // Bonus for matching first character
      if (queryIndex === 0 && i === 0) {
        score += 3
      }

      queryIndex++
    } else {
      consecutive = 0
    }
  }

  // All query characters must match
  if (queryIndex < q.length) return 0

  return score
}

export function fuzzySearch(
  query: string,
  items: { path: string; name: string }[],
  maxResults = 50,
): SearchResult[] {
  if (!query.trim()) return items.slice(0, maxResults).map((item) => ({ ...item, score: 0 }))

  const results: SearchResult[] = []

  for (const item of items) {
    // Score against both name and full path, take the better score
    const nameScore = fuzzyScore(query, item.name)
    const pathScore = fuzzyScore(query, item.path)
    const score = Math.max(nameScore * 1.5, pathScore) // Boost name matches

    if (score > 0) {
      results.push({ ...item, score })
    }
  }

  results.sort((a, b) => b.score - a.score)
  return results.slice(0, maxResults)
}
```

- [ ] **Step 2: Create `src/renderer/src/lib/path-utils.ts`**

```typescript
export function basename(path: string): string {
  return path.split(/[/\\]/).pop() || path
}

export function shortenPath(path: string, maxLen = 40): string {
  if (path.length <= maxLen) return path
  const parts = path.split(/[/\\]/)
  if (parts.length <= 2) return path
  return `.../${parts.slice(-2).join('/')}`
}
```

- [ ] **Step 3: Commit**

```bash
git add src/renderer/src/lib/fuzzy-search.ts src/renderer/src/lib/path-utils.ts
git commit -m "feat: add fuzzy search scoring and path utils"
```

---

## Task 14: WelcomeView Component

**Files:**

- Create: `src/renderer/src/components/WelcomeView.tsx`

- [ ] **Step 1: Create `src/renderer/src/components/WelcomeView.tsx`**

```tsx
import { useCallback, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore } from '../store/app-store'

export function WelcomeView() {
  const setActiveFile = useAppStore((s) => s.setActiveFile)
  const setOpenFolder = useAppStore((s) => s.setOpenFolder)
  const queryClient = useQueryClient()
  const [isDragOver, setIsDragOver] = useState(false)

  const handleOpenFile = useCallback(async () => {
    const result = await window.api.openFileDialog()
    if (result) {
      setActiveFile(result)
      queryClient.invalidateQueries({ queryKey: ['recents'] })
    }
  }, [setActiveFile, queryClient])

  const handleOpenFolder = useCallback(async () => {
    const result = await window.api.openFolderDialog()
    if (result) {
      setOpenFolder(result.path, result.tree)
    }
  }, [setOpenFolder])

  const handleDrop = useCallback(
    async (e: React.DragEvent) => {
      e.preventDefault()
      setIsDragOver(false)

      const files = Array.from(e.dataTransfer.files)
      const mdFile = files.find(
        (f) => f.name.endsWith('.md') || f.name.endsWith('.markdown') || f.name.endsWith('.mdx'),
      )

      if (mdFile) {
        const content = await window.api.readFile(mdFile.path)
        setActiveFile({ path: mdFile.path, content })
        queryClient.invalidateQueries({ queryKey: ['recents'] })
      }
    },
    [setActiveFile, queryClient],
  )

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    setIsDragOver(true)
  }, [])

  const handleDragLeave = useCallback(() => {
    setIsDragOver(false)
  }, [])

  return (
    <div
      className="welcome-view"
      onDrop={handleDrop}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
    >
      <div className={`drop-hint ${isDragOver ? 'drag-over' : ''}`}>
        <h2>mdview</h2>
        <p>Drop a markdown file here to view it</p>
      </div>
      <div style={{ display: 'flex', gap: 8 }}>
        <button onClick={handleOpenFile}>Open File</button>
        <button onClick={handleOpenFolder}>Open Folder</button>
      </div>
    </div>
  )
}
```

- [ ] **Step 2: Commit**

```bash
git add src/renderer/src/components/WelcomeView.tsx
git commit -m "feat: add WelcomeView with drag-drop and open buttons"
```

---

## Task 15: MarkdownView Component

**Files:**

- Create: `src/renderer/src/components/MarkdownView.tsx`

- [ ] **Step 1: Create `src/renderer/src/components/MarkdownView.tsx`**

```tsx
import { useEffect, useRef, useState } from 'react'
import { initMarkdown, renderMarkdown, type RenderResult } from '../lib/markdown'
import { initMermaid, renderMermaidBlocks, updateMermaidTheme } from '../lib/mermaid'

interface MarkdownViewProps {
  content: string
}

export function MarkdownView({ content }: MarkdownViewProps) {
  const [html, setHtml] = useState('')
  const [ready, setReady] = useState(false)
  const mermaidBlocksRef = useRef<RenderResult['mermaidBlocks']>([])

  // Initialize rendering libraries
  useEffect(() => {
    const isDark = document.documentElement.classList.contains('dark')
    initMarkdown().then(() => {
      initMermaid(isDark)
      setReady(true)
    })
  }, [])

  // Render markdown when content changes
  useEffect(() => {
    if (!ready || !content) return

    const result = renderMarkdown(content)
    setHtml(result.html)
    mermaidBlocksRef.current = result.mermaidBlocks
  }, [content, ready])

  // Render mermaid diagrams after HTML is injected into the DOM
  useEffect(() => {
    if (mermaidBlocksRef.current.length > 0) {
      renderMermaidBlocks(mermaidBlocksRef.current)
    }
  }, [html])

  // Listen for theme changes to re-render
  useEffect(() => {
    const observer = new MutationObserver(() => {
      const isDark = document.documentElement.classList.contains('dark')
      updateMermaidTheme(isDark)
      if (ready && content) {
        const result = renderMarkdown(content)
        setHtml(result.html)
        mermaidBlocksRef.current = result.mermaidBlocks
      }
    })

    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class'],
    })

    return () => observer.disconnect()
  }, [ready, content])

  if (!ready) {
    return (
      <div className="content-area" style={{ opacity: 0.5 }}>
        Loading...
      </div>
    )
  }

  return <div className="content-area markdown-body" dangerouslySetInnerHTML={{ __html: html }} />
}
```

Note: `dangerouslySetInnerHTML` is safe here because the content comes from locally-read markdown files rendered through md4x/shiki — the same trust model as the original native mdview app. This is not user-submitted web content.

- [ ] **Step 2: Commit**

```bash
git add src/renderer/src/components/MarkdownView.tsx
git commit -m "feat: add MarkdownView with md4x, shiki, and mermaid rendering"
```

---

## Task 16: RecentsList Component

**Files:**

- Create: `src/renderer/src/components/RecentsList.tsx`

- [ ] **Step 1: Create `src/renderer/src/components/RecentsList.tsx`**

```tsx
import { useCallback } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useRecents } from '../hooks/useRecents'
import { useAppStore } from '../store/app-store'
import { basename } from '../lib/path-utils'

export function RecentsList() {
  const { data: recents = [] } = useRecents()
  const setActiveFile = useAppStore((s) => s.setActiveFile)
  const activeFile = useAppStore((s) => s.activeFile)
  const queryClient = useQueryClient()

  const handleClick = useCallback(
    async (path: string) => {
      const content = await window.api.readFile(path)
      setActiveFile({ path, content })
      queryClient.invalidateQueries({ queryKey: ['recents'] })
    },
    [setActiveFile, queryClient],
  )

  const handleContextMenu = useCallback((path: string) => {
    window.api.showInFolder(path)
  }, [])

  if (recents.length === 0) {
    return null
  }

  return (
    <>
      <div className="sidebar-section-label">Recents</div>
      <div className="sidebar-list">
        {recents.map((path) => (
          <div
            key={path}
            className={`sidebar-item ${activeFile?.path === path ? 'active' : ''}`}
            onClick={() => handleClick(path)}
            onContextMenu={() => handleContextMenu(path)}
            title={path}
          >
            {basename(path)}
          </div>
        ))}
      </div>
    </>
  )
}
```

- [ ] **Step 2: Commit**

```bash
git add src/renderer/src/components/RecentsList.tsx
git commit -m "feat: add RecentsList component"
```

---

## Task 17: FolderTree Component

**Files:**

- Create: `src/renderer/src/components/FolderTree.tsx`

- [ ] **Step 1: Create `src/renderer/src/components/FolderTree.tsx`**

```tsx
import { useCallback, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore } from '../store/app-store'

interface TreeNode {
  name: string
  path: string
  isDirectory: boolean
  children?: TreeNode[]
}

function TreeItem({
  node,
  depth,
  activeFilePath,
  onFileClick,
}: {
  node: TreeNode
  depth: number
  activeFilePath: string | null
  onFileClick: (path: string) => void
}) {
  const [expanded, setExpanded] = useState(depth === 0)

  if (node.isDirectory) {
    return (
      <>
        <div
          className={`sidebar-item indent-${Math.min(depth, 3)}`}
          onClick={() => setExpanded(!expanded)}
        >
          <span style={{ fontSize: 10, marginRight: 4 }}>{expanded ? '\u25BE' : '\u25B8'}</span>
          {node.name}/
        </div>
        {expanded &&
          node.children?.map((child) => (
            <TreeItem
              key={child.path}
              node={child}
              depth={depth + 1}
              activeFilePath={activeFilePath}
              onFileClick={onFileClick}
            />
          ))}
      </>
    )
  }

  return (
    <div
      className={`sidebar-item indent-${Math.min(depth, 3)} ${
        activeFilePath === node.path ? 'active' : ''
      }`}
      onClick={() => onFileClick(node.path)}
      title={node.path}
    >
      {node.name}
    </div>
  )
}

export function FolderTree() {
  const folderTree = useAppStore((s) => s.folderTree)
  const openFolderPath = useAppStore((s) => s.openFolderPath)
  const activeFile = useAppStore((s) => s.activeFile)
  const setActiveFile = useAppStore((s) => s.setActiveFile)
  const queryClient = useQueryClient()

  const handleFileClick = useCallback(
    async (path: string) => {
      const content = await window.api.readFile(path)
      setActiveFile({ path, content })
      queryClient.invalidateQueries({ queryKey: ['recents'] })
    },
    [setActiveFile, queryClient],
  )

  if (!openFolderPath || folderTree.length === 0) return null

  const folderName = openFolderPath.split(/[/\\]/).pop() || openFolderPath

  return (
    <>
      <div className="sidebar-divider" />
      <div
        className="sidebar-section-label"
        style={{ display: 'flex', justifyContent: 'space-between' }}
      >
        <span>Folder</span>
        <span style={{ opacity: 0.6, textTransform: 'none', fontWeight: 400 }}>{folderName}</span>
      </div>
      <div className="sidebar-list">
        {folderTree.map((node) => (
          <TreeItem
            key={node.path}
            node={node}
            depth={0}
            activeFilePath={activeFile?.path ?? null}
            onFileClick={handleFileClick}
          />
        ))}
      </div>
    </>
  )
}
```

- [ ] **Step 2: Commit**

```bash
git add src/renderer/src/components/FolderTree.tsx
git commit -m "feat: add FolderTree component with expand/collapse"
```

---

## Task 18: Sidebar Component

**Files:**

- Create: `src/renderer/src/components/Sidebar.tsx`

- [ ] **Step 1: Create `src/renderer/src/components/Sidebar.tsx`**

```tsx
import { useCallback, useRef } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore } from '../store/app-store'
import { RecentsList } from './RecentsList'
import { FolderTree } from './FolderTree'

export function Sidebar() {
  const sidebarWidth = useAppStore((s) => s.sidebarWidth)
  const setSidebarWidth = useAppStore((s) => s.setSidebarWidth)
  const setCommandPaletteOpen = useAppStore((s) => s.setCommandPaletteOpen)
  const setActiveFile = useAppStore((s) => s.setActiveFile)
  const setOpenFolder = useAppStore((s) => s.setOpenFolder)
  const queryClient = useQueryClient()
  const resizing = useRef(false)

  const handleOpenFile = useCallback(async () => {
    const result = await window.api.openFileDialog()
    if (result) {
      setActiveFile(result)
      queryClient.invalidateQueries({ queryKey: ['recents'] })
    }
  }, [setActiveFile, queryClient])

  const handleOpenFolder = useCallback(async () => {
    const result = await window.api.openFolderDialog()
    if (result) {
      setOpenFolder(result.path, result.tree)
    }
  }, [setOpenFolder])

  const handleResizeStart = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault()
      resizing.current = true
      const startX = e.clientX
      const startWidth = sidebarWidth

      const handleMouseMove = (e: MouseEvent) => {
        if (!resizing.current) return
        const newWidth = Math.max(200, Math.min(400, startWidth + (e.clientX - startX)))
        setSidebarWidth(newWidth)
      }

      const handleMouseUp = () => {
        resizing.current = false
        window.api.saveAppState({ sidebarWidth })
        document.removeEventListener('mousemove', handleMouseMove)
        document.removeEventListener('mouseup', handleMouseUp)
      }

      document.addEventListener('mousemove', handleMouseMove)
      document.addEventListener('mouseup', handleMouseUp)
    },
    [sidebarWidth, setSidebarWidth],
  )

  const isMac = navigator.platform.includes('Mac')
  const modKey = isMac ? '\u2318' : 'Ctrl+'

  return (
    <>
      <div className="sidebar" style={{ width: sidebarWidth }}>
        <div className="sidebar-header">
          <div className="quick-open-trigger" onClick={() => setCommandPaletteOpen(true)}>
            <span style={{ opacity: 0.5 }}>&#x1F50D;</span>
            <span>Quick Open</span>
            <span className="shortcut">{modKey}K</span>
          </div>
        </div>

        <RecentsList />
        <FolderTree />

        <div className="sidebar-footer">
          <button onClick={handleOpenFile}>Open File</button>
          <button onClick={handleOpenFolder}>Open Folder</button>
        </div>
      </div>
      <div className="resize-handle" onMouseDown={handleResizeStart} />
    </>
  )
}
```

- [ ] **Step 2: Commit**

```bash
git add src/renderer/src/components/Sidebar.tsx
git commit -m "feat: add Sidebar with resize handle, quick open trigger, and footer"
```

---

## Task 19: CommandPalette Component

**Files:**

- Create: `src/renderer/src/components/CommandPalette.tsx`

- [ ] **Step 1: Create `src/renderer/src/components/CommandPalette.tsx`**

```tsx
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore } from '../store/app-store'
import { useRecents } from '../hooks/useRecents'
import { fuzzySearch, type SearchResult } from '../lib/fuzzy-search'
import { basename } from '../lib/path-utils'

function flattenTree(nodes: any[], result: { path: string; name: string }[] = []) {
  for (const node of nodes) {
    if (node.isDirectory && node.children) {
      flattenTree(node.children, result)
    } else if (!node.isDirectory) {
      result.push({ path: node.path, name: node.name })
    }
  }
  return result
}

export function CommandPalette() {
  const commandPaletteOpen = useAppStore((s) => s.commandPaletteOpen)
  const setCommandPaletteOpen = useAppStore((s) => s.setCommandPaletteOpen)
  const setActiveFile = useAppStore((s) => s.setActiveFile)
  const folderTree = useAppStore((s) => s.folderTree)
  const { data: recents = [] } = useRecents()
  const queryClient = useQueryClient()

  const [query, setQuery] = useState('')
  const [selectedIndex, setSelectedIndex] = useState(0)
  const inputRef = useRef<HTMLInputElement>(null)

  // Build combined file list: folder files first, then recents (deduped)
  const allFiles = useMemo(() => {
    const folderFiles = flattenTree(folderTree)
    const folderPaths = new Set(folderFiles.map((f) => f.path))
    const recentFiles = recents
      .filter((path) => !folderPaths.has(path))
      .map((path) => ({ path, name: basename(path) }))
    return [...folderFiles, ...recentFiles]
  }, [folderTree, recents])

  const results = useMemo(() => fuzzySearch(query, allFiles), [query, allFiles])

  // Reset on open
  useEffect(() => {
    if (commandPaletteOpen) {
      setQuery('')
      setSelectedIndex(0)
      setTimeout(() => inputRef.current?.focus(), 0)
    }
  }, [commandPaletteOpen])

  const selectFile = useCallback(
    async (path: string) => {
      setCommandPaletteOpen(false)
      const content = await window.api.readFile(path)
      setActiveFile({ path, content })
      queryClient.invalidateQueries({ queryKey: ['recents'] })
    },
    [setCommandPaletteOpen, setActiveFile, queryClient],
  )

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      switch (e.key) {
        case 'ArrowDown':
          e.preventDefault()
          setSelectedIndex((i) => Math.min(i + 1, results.length - 1))
          break
        case 'ArrowUp':
          e.preventDefault()
          setSelectedIndex((i) => Math.max(i - 1, 0))
          break
        case 'Enter':
          e.preventDefault()
          if (results[selectedIndex]) {
            selectFile(results[selectedIndex].path)
          }
          break
        case 'Escape':
          e.preventDefault()
          setCommandPaletteOpen(false)
          break
      }
    },
    [results, selectedIndex, selectFile, setCommandPaletteOpen],
  )

  if (!commandPaletteOpen) return null

  return (
    <div className="command-palette-overlay" onClick={() => setCommandPaletteOpen(false)}>
      <div className="command-palette" onClick={(e) => e.stopPropagation()}>
        <input
          ref={inputRef}
          className="command-palette-input"
          placeholder="Search files..."
          value={query}
          onChange={(e) => {
            setQuery(e.target.value)
            setSelectedIndex(0)
          }}
          onKeyDown={handleKeyDown}
        />
        <div className="command-palette-results">
          {results.length === 0 && query && (
            <div className="command-palette-empty">No matching files</div>
          )}
          {results.map((result, index) => (
            <div
              key={result.path}
              className={`command-palette-item ${index === selectedIndex ? 'selected' : ''}`}
              onClick={() => selectFile(result.path)}
              onMouseEnter={() => setSelectedIndex(index)}
            >
              <span className="filename">{result.name}</span>
              <span className="filepath">{result.path}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}
```

- [ ] **Step 2: Commit**

```bash
git add src/renderer/src/components/CommandPalette.tsx
git commit -m "feat: add CommandPalette with fuzzy search and keyboard navigation"
```

---

## Task 20: Wire Up App.tsx with All Components

**Files:**

- Modify: `src/renderer/src/App.tsx`
- Modify: `src/renderer/src/main.tsx`

- [ ] **Step 1: Update `src/renderer/src/main.tsx` with QueryClientProvider**

```tsx
import React from 'react'
import ReactDOM from 'react-dom/client'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import App from './App'
import './assets/styles/index.css'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: false,
      refetchOnWindowFocus: false,
    },
  },
})

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </React.StrictMode>,
)
```

- [ ] **Step 2: Update `src/renderer/src/App.tsx`**

```tsx
import { useCallback, useEffect } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore } from './store/app-store'
import { useTheme } from './hooks/useTheme'
import { useFolderTree } from './hooks/useFolderTree'
import { Sidebar } from './components/Sidebar'
import { MarkdownView } from './components/MarkdownView'
import { WelcomeView } from './components/WelcomeView'
import { CommandPalette } from './components/CommandPalette'

function App(): React.JSX.Element {
  const activeFile = useAppStore((s) => s.activeFile)
  const sidebarOpen = useAppStore((s) => s.sidebarOpen)
  const toggleSidebar = useAppStore((s) => s.toggleSidebar)
  const setActiveFile = useAppStore((s) => s.setActiveFile)
  const setOpenFolder = useAppStore((s) => s.setOpenFolder)
  const setSidebarWidth = useAppStore((s) => s.setSidebarWidth)
  const openFolderPath = useAppStore((s) => s.openFolderPath)
  const setCommandPaletteOpen = useAppStore((s) => s.setCommandPaletteOpen)
  const updateActiveFileContent = useAppStore((s) => s.updateActiveFileContent)
  const queryClient = useQueryClient()

  // Theme handling
  useTheme()

  // Folder tree query (auto-watches via IPC events)
  useFolderTree(openFolderPath)

  // Restore app state on mount
  useEffect(() => {
    window.api.getAppState().then((state) => {
      if (state.sidebarWidth) setSidebarWidth(state.sidebarWidth)
      if (state.lastFolder) {
        window.api.readFolderTree(state.lastFolder).then((tree) => {
          setOpenFolder(state.lastFolder!, tree)
        })
      }
    })
  }, [setSidebarWidth, setOpenFolder])

  // Handle menu events
  useEffect(() => {
    const unsubs = [
      window.api.onMenuOpenFile(async () => {
        const result = await window.api.openFileDialog()
        if (result) {
          setActiveFile(result)
          queryClient.invalidateQueries({ queryKey: ['recents'] })
        }
      }),
      window.api.onMenuOpenFolder(async () => {
        const result = await window.api.openFolderDialog()
        if (result) {
          setOpenFolder(result.path, result.tree)
        }
      }),
      window.api.onFileOpened((file) => {
        setActiveFile(file)
        queryClient.invalidateQueries({ queryKey: ['recents'] })
      }),
      window.api.onFileChanged((content) => {
        updateActiveFileContent(content)
      }),
    ]
    return () => unsubs.forEach((fn) => fn())
  }, [setActiveFile, setOpenFolder, updateActiveFileContent, queryClient])

  // Global keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey
      if (mod && e.key === 'k') {
        e.preventDefault()
        setCommandPaletteOpen(true)
      }
      if (mod && e.key === 'b') {
        e.preventDefault()
        toggleSidebar()
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [setCommandPaletteOpen, toggleSidebar])

  // Drag and drop on content area
  const handleDrop = useCallback(
    async (e: React.DragEvent) => {
      e.preventDefault()
      const files = Array.from(e.dataTransfer.files)
      const mdFile = files.find(
        (f) => f.name.endsWith('.md') || f.name.endsWith('.markdown') || f.name.endsWith('.mdx'),
      )
      if (mdFile) {
        const content = await window.api.readFile(mdFile.path)
        setActiveFile({ path: mdFile.path, content })
        queryClient.invalidateQueries({ queryKey: ['recents'] })
      }
    },
    [setActiveFile, queryClient],
  )

  return (
    <div className="app-layout" onDrop={handleDrop} onDragOver={(e) => e.preventDefault()}>
      {sidebarOpen && <Sidebar />}
      {activeFile ? <MarkdownView content={activeFile.content} /> : <WelcomeView />}
      <CommandPalette />
    </div>
  )
}

export default App
```

- [ ] **Step 3: Verify the full app works**

```bash
npm run dev
```

Expected: App starts with sidebar (Quick Open bar, empty recents), content area shows WelcomeView. Click "Open File" to open a `.md` file and verify it renders with syntax highlighting. Cmd/Ctrl+K opens command palette. Cmd/Ctrl+B toggles sidebar.

- [ ] **Step 4: Commit**

```bash
git add src/renderer/src/App.tsx src/renderer/src/main.tsx
git commit -m "feat: wire up all components in App with keyboard shortcuts and drag-drop"
```

---

## Task 21: Gitignore and Final Polish

**Files:**

- Create: `.gitignore`
- Modify: `package.json`

- [ ] **Step 1: Create `.gitignore`**

```
node_modules/
out/
dist/
.superpowers/
*.log
```

- [ ] **Step 2: Update `package.json` name and description**

In `package.json`, ensure:

- `"name": "mdview-electron"`
- `"description": "A modern cross-platform markdown viewer"`

- [ ] **Step 3: Final full test**

```bash
npm run dev
```

Test the following:

1. App launches with sidebar + welcome view
2. Open File button selects a `.md` file and renders it with syntax highlighting
3. Open Folder shows a filtered tree and clicking a file renders it
4. Cmd/Ctrl+K opens the command palette, fuzzy search works, Enter selects a file
5. Cmd/Ctrl+B toggles the sidebar
6. Drag a `.md` file onto the window and it renders
7. Edit the open file externally and the content auto-updates
8. Resize the sidebar, close and reopen the app, and verify the width is persisted
9. Toggle system dark mode and verify the theme updates live

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: add gitignore and finalize package metadata"
```

---

## Summary

| Task | What it builds                                 |
| ---- | ---------------------------------------------- |
| 1    | Project scaffold with electron-vite + all deps |
| 2    | Fonts, CSS theme system, app shell layout      |
| 3    | Typed preload bridge (all IPC channels)        |
| 4    | electron-store persistence layer               |
| 5    | File service (read, dialog, watch)             |
| 6    | Folder service (scan, dialog, watch)           |
| 7    | IPC handler registration                       |
| 8    | Native application menu                        |
| 9    | Main process entry with window lifecycle       |
| 10   | Zustand store                                  |
| 11   | TanStack Query hooks + IPC event hook          |
| 12   | Markdown rendering (md4x + shiki + mermaid)    |
| 13   | Fuzzy search and path utils                    |
| 14   | WelcomeView component                          |
| 15   | MarkdownView component                         |
| 16   | RecentsList component                          |
| 17   | FolderTree component                           |
| 18   | Sidebar component                              |
| 19   | CommandPalette component                       |
| 20   | Wire everything in App.tsx                     |
| 21   | Gitignore and final polish                     |
