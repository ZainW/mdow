# Multi-Window CLI Support Spec

This document specifies the design and implementation details for adding globally-accessible CLI support (`mdow <path>`) with a true multi-window single-instance architecture in Mdow.

## Status: Approved

---

## 1. Goals & Requirements

*   **Global Command:** A command-line script/binary named `mdow` that can be run from any terminal directory.
*   **Path Support:** The command must accept both directories (`mdow .`, `mdow /some/folder`) and individual markdown files (`mdow README.md`).
*   **True Multi-Window UX:** 
    *   If Mdow is **not** running: Launch the app with a single window displaying the targeted path.
    *   If Mdow **is** running: Communicate with the running process to spawn a **new independent window** displaying the targeted path.
    *   If the target path is already open in an existing window: Focus that window instead of opening a duplicate.
*   **Relative Path Resolution:** The CLI utility must resolve relative paths (`.`, `..`, `./docs`) to absolute paths *before* launching/communicating with Electron, ensuring the app resolves files/folders from the user's terminal workspace context instead of Electron's context.

---

## 2. Technical Architecture & Component Breakdown

### A. CLI Wrapper Script (`bin/mdow`)
A lightweight, robust shell script in the repository root (`bin/mdow`) that can be symlinked into the system path (e.g. `/usr/local/bin/mdow`).

*   **Logic:**
    1.  Iterates over arguments, checking if they exist on disk.
    2.  Resolves existing paths to absolute paths.
    3.  Launches or forwards arguments to the packaged application.
*   **macOS Command:** `open -a "Mdow" --args $ABS_PATHS`
*   **Linux/Windows Compatibility:** Cascades to direct binary calls on other platforms.

### B. Main Process Window Manager (`apps/desktop/src/main/index.ts` & `ipc.ts`)
We will transition the main process from a single `mainWindow` to a multi-window manager.

*   **Window Management State:**
    ```typescript
    const windows = new Set<BrowserWindow>()
    // Tracks which window is open to which path (folder path or active file path)
    const windowPaths = new Map<BrowserWindow, string>()
    ```
*   **Refactoring `mainWindow` references:**
    *   Instead of a single global `mainWindow: BrowserWindow | null`, we will track all active windows in `windows`.
    *   `getMainWindow()` helper will return the currently focused or last active window, ensuring backward compatibility for global shortcuts or menu clicks.
    *   When all windows are closed, exit on Linux/Windows and clean up watchers.
*   **IPC & Event Dispatching:**
    *   Currently, functions like `registerIpcHandlers` accept a single-window getter.
    *   We will refactor IPC handlers inside `ipc.ts` to dynamically retrieve the correct window executing the request via `BrowserWindow.fromWebContents(event.sender)` instead of using the global `getMainWindow` getter whenever possible.
*   **Spawning Windows with Arguments:**
    *   `createWindow(targetPath?: string)` will load the renderer HTML.
    *   To pass `targetPath` cleanly on spawn, we will append it to the window's query parameters:
        *   In Development: `loadURL(url + "?openPath=" + encodeURIComponent(targetPath))`
        *   In Production: `loadFile(htmlPath, { query: { openPath: targetPath } })`
*   **Second Instance Interception:**
    *   In the `second-instance` handler:
        1.  Extract the target path from incoming `argv`.
        2.  Check if any existing window in `windowPaths` matches this path.
        3.  If found: Focus that window.
        4.  If not: Create a new window using `createWindow(targetPath)`.

### C. Frontend Router & Initializer (`apps/desktop/src/renderer/src/hooks/useAppInit.ts`)
The React application needs to check for the presence of the `openPath` query parameter and initialize accordingly.

*   **Initialization Logic:**
    1.  Parse `window.location.search` for `openPath`.
    2.  If `openPath` exists:
        *   Bypass the standard session-restoration state (i.e. do not automatically restore `lastFolder` or previous `sessionTabs`).
        *   Determine if `openPath` is a directory or a file.
        *   If directory: Call `window.api.openFolderPath(openPath)` and load the folder tree.
        *   If file: Call `window.api.readFile(openPath)` and open a tab for that file.
    3.  If no `openPath` is present:
        *   Fall back to standard behavior (load `lastFolder` and reopen the active session tabs).

---

## 3. Data Flow Diagram

```
[ Terminal ] ────> ( $ mdow . )
                       │
                       ▼
               [ bin/mdow Script ]  (Resolves "." to absolute path "/Users/zain/projects/mdow")
                       │
                       ▼
           [ Electron Main Process ] (second-instance event triggered)
                       │
             ┌─────────┴─────────┐
             ▼                   ▼
    [ Already Open? ]       [ Not Open? ]
      Focus Window       Create New Window with "?openPath=/Users/zain/projects/mdow"
                                 │
                                 ▼
                         [ React App Load ]
                                 │
                                 ▼
                     ( Reads openPath param )
                                 │
                                 ▼
                       ( Loads "/Users/zain/projects/mdow" )
```

---

## 4. Verification & Testing Plan

*   **Development Testing:**
    *   Verify we can run `pnpm run --filter desktop dev -- <path>` and it successfully spawns/targets windows.
    *   Verify opening files vs directories.
*   **Unit Tests:**
    *   Add tests in `apps/desktop/src/main/index.test.ts` or `ipc.test.ts` checking `openPath` argument parsing and window tracking.
    *   Add tests in `apps/desktop/src/renderer/src/dev/open-dev-workspace.test.ts` or a new init test verifying `openPath` query param routing.
