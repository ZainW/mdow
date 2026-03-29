# mdview-electron — Design Spec

A modern Electron-based markdown viewer, porting the native macOS mdview app to a cross-platform desktop application using electron-vite, React, and the same md4x rendering pipeline.

## Scope

**v1 is a viewer only.** Editing capabilities (split-pane editor) will come in a future iteration. The architecture accommodates this but nothing editing-related is built in v1.

## Architecture

### Process Model

Three-layer Electron architecture:

- **Main process** — window management, file system access (read, dialogs), native menus, folder watching (chokidar), recent files persistence (electron-store), IPC handlers.
- **Preload script** — secure bridge via `contextBridge`. Exposes a typed `window.api` object to the renderer. All IPC channels are type-safe end to end.
- **Renderer process** — React 19 app. Zustand for app state, TanStack Query for async IPC calls, TanStack Virtual for sidebar list virtualization. md4x + shiki + mermaid for markdown rendering.

### Tech Stack

| Layer         | Technology                                                                |
| ------------- | ------------------------------------------------------------------------- |
| Build tooling | electron-vite                                                             |
| Main process  | TypeScript, chokidar, electron-store                                      |
| Preload       | TypeScript, contextBridge                                                 |
| Renderer      | React 19, TypeScript                                                      |
| State         | Zustand (app state), TanStack Query (async/IPC), TanStack Virtual (lists) |
| Markdown      | md4x (WASM parser), shiki (syntax highlighting), mermaid (diagrams)       |
| Fonts         | Inter Variable (body), Geist Mono Variable (code)                         |
| Platforms     | macOS, Windows, Linux                                                     |

### Window Chrome

Native OS title bar on all platforms. No custom/frameless window chrome.

## UI Layout

Single window with two regions:

### Sidebar (left)

Unified sidebar showing both sections at all times (no tabs/segmented picker):

1. **Quick Open bar** at top — displays search icon, "Quick Open" text, and Cmd/Ctrl+K hint. Clicking it or pressing the shortcut opens the command palette overlay.
2. **Recents section** — labeled "Recents", lists up to 20 recently opened files with short display names.
3. **Divider** — visual separator between sections.
4. **Folder section** — labeled "Folder" with the current folder path. Hierarchical tree view showing only directories that transitively contain `.md` files. Expandable/collapsible nodes.
5. **Footer** — "Open File" and "Open Folder" buttons.

Sidebar properties:

- Resizable width via drag handle, persisted to electron-store.
- Collapsible with Cmd/Ctrl+B.
- Virtualized lists via TanStack Virtual for performance with large folders.

### Content Area (right)

Rendered markdown output. Same styling as the original mdview: heading borders, blockquote borders, striped tables, rounded code blocks with syntax highlighting, centered mermaid diagrams.

When no file is open, shows a welcome view with drag-drop hint and open file/folder buttons.

## Features

### File Opening

- **Open file dialog** — Cmd/Ctrl+O. Native file picker filtered to `.md` files. Returns path + content.
- **Open folder dialog** — Cmd/Ctrl+Shift+O. Native folder picker. Scans recursively for `.md` files, builds filtered tree.
- **Drag and drop** — Drop `.md` files onto the window (welcome view or content area).
- **CLI** — `mdview file.md` opens the app with the file. Passed as process arg, handled in main process.
- **File association** — Register as handler for `.md` files on all platforms.

### Command Palette

- Triggered by Cmd/Ctrl+K or clicking the Quick Open bar in the sidebar.
- Modal overlay with search input.
- Fuzzy matches against filenames and paths from both recents and folder tree (folder files first, deduped).
- Scoring: bonuses for word boundary matches, consecutive character matches, first character match.
- Keyboard navigation: Up/Down arrows, Enter to select, Escape to close.
- Maximum 50 results displayed.

### Markdown Rendering

Same pipeline as the original mdview:

1. **Parse** — md4x WASM parser (GFM syntax).
2. **Highlight** — shiki with 31 languages (JavaScript, TypeScript, Python, Rust, Go, Java, C, C++, C#, Ruby, Swift, Kotlin, HTML, CSS, JSON, YAML, TOML, XML, Markdown, SQL, Bash, Shell Script, Diff, GraphQL, Dockerfile, Lua, Zig, Elixir, Haskell, OCaml, JSX, TSX, PHP). Themes: GitHub Light, GitHub Dark.
3. **Diagrams** — mermaid for flowcharts, sequence diagrams, etc. Loose security level. Theme synced with system appearance.
4. **Render** — inject processed HTML into the React content area.

### Theme

- Follows system light/dark preference via `nativeTheme.shouldUseDarkColors`.
- Live-updates when OS theme changes (main process listens to `nativeTheme.on('updated')`, sends `theme:changed` to renderer).
- Syncs shiki theme (GitHub Light / GitHub Dark) and mermaid theme on change.
- CSS variables for all colors, toggled via a class on the root element.

### File Watching

- **Active file** — main process watches the currently open file via chokidar. On change, reads new content and sends `file:changed` to renderer. TanStack Query invalidates the file query → markdown re-renders.
- **Open folder** — main process watches the folder for added/removed/renamed `.md` files. On change (debounced), rescans and sends `folder:changed` to renderer. TanStack Query invalidates the folder query → sidebar updates.

### Persistence (electron-store)

- Recent files list (max 20, stored as absolute paths).
- Last opened folder path.
- Window size and position (restored on launch).
- Sidebar width.
- Restore last opened folder on app launch.

### Keyboard Shortcuts

| Shortcut         | Action          |
| ---------------- | --------------- |
| Cmd/Ctrl+O       | Open file       |
| Cmd/Ctrl+Shift+O | Open folder     |
| Cmd/Ctrl+K       | Command palette |
| Cmd/Ctrl+B       | Toggle sidebar  |

All shortcuts are platform-aware (Cmd on macOS, Ctrl on Windows/Linux). Registered as both native menu accelerators and handled in the renderer for the command palette.

## IPC Design

### Renderer → Main (invoke/handle)

| Channel                | Purpose                                               |
| ---------------------- | ----------------------------------------------------- |
| `file:open-dialog`     | Opens native file picker, returns `{ path, content }` |
| `file:read`            | Reads file at path, returns content string            |
| `folder:open-dialog`   | Opens native folder picker, returns tree structure    |
| `folder:read-tree`     | Scans folder recursively, returns filtered tree       |
| `store:get-recents`    | Returns recent files list                             |
| `store:get-state`      | Returns persisted app state                           |
| `shell:show-in-folder` | Opens file location in OS file manager                |

### Main → Renderer (send/on)

| Channel            | Payload                                                       |
| ------------------ | ------------------------------------------------------------- |
| `file:changed`     | New content string for the watched file                       |
| `folder:changed`   | Updated tree structure                                        |
| `theme:changed`    | `{ isDark: boolean }`                                         |
| `menu:open-file`   | User clicked Open File in native menu                         |
| `menu:open-folder` | User clicked Open Folder in native menu                       |
| `file:opened`      | File opened via CLI or file association — `{ path, content }` |

### TanStack Query Integration

- **File content** — `queryKey: ['file', filePath]`, `queryFn` calls `window.api.readFile(path)`. Invalidated on `file:changed` event.
- **Folder tree** — `queryKey: ['folder', folderPath]`, `queryFn` calls `window.api.readFolderTree(path)`. Invalidated on `folder:changed` event.
- **Recents** — `queryKey: ['recents']`. Invalidated when a file is opened.

## Project Structure

```
mdview-electron/
├── electron.vite.config.ts
├── package.json
├── tsconfig.json
├── tsconfig.node.json
├── tsconfig.web.json
│
├── src/
│   ├── main/
│   │   ├── index.ts              # App entry, window creation
│   │   ├── menu.ts               # Native menu setup
│   │   ├── ipc.ts                # IPC handler registration
│   │   ├── file-service.ts       # File read, dialogs
│   │   ├── folder-service.ts     # Folder tree, chokidar watch
│   │   └── store.ts              # electron-store config
│   │
│   ├── preload/
│   │   └── index.ts              # contextBridge + typed API
│   │
│   └── renderer/
│       ├── index.html
│       ├── main.tsx              # React entry
│       ├── App.tsx               # Root layout (sidebar + content)
│       │
│       ├── components/
│       │   ├── Sidebar.tsx
│       │   ├── RecentsList.tsx
│       │   ├── FolderTree.tsx
│       │   ├── MarkdownView.tsx
│       │   ├── CommandPalette.tsx
│       │   └── WelcomeView.tsx
│       │
│       ├── hooks/
│       │   ├── useFileContent.ts
│       │   ├── useFolderTree.ts
│       │   ├── useRecents.ts
│       │   └── useTheme.ts
│       │
│       ├── store/
│       │   └── app-store.ts      # Zustand store
│       │
│       ├── lib/
│       │   ├── markdown.ts       # md4x + shiki + mermaid init & render
│       │   └── fuzzy-search.ts   # Command palette scoring
│       │
│       └── assets/
│           ├── fonts/            # Inter, Geist Mono (woff2)
│           └── styles/
│               └── index.css     # Global styles + markdown CSS
│
└── resources/                    # App icons per platform
    ├── icon.icns
    ├── icon.ico
    └── icon.png
```

## Explicitly Out of Scope (v1)

- Editing / split-pane editor
- Multiple windows / tabs
- Auto-updates
- Code signing / distribution packaging
- Printing / PDF export
- Custom themes beyond system light/dark
- Plugin system
