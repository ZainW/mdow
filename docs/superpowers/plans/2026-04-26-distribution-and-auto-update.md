# Distribution and Auto-Update Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the website download page to live GitHub Releases, finish the in-app updater for Win/Linux (Mac is no-op + "open releases page" + Homebrew Cask), and add a Homebrew tap for one-command Mac install.

**Architecture:** GitHub Releases is the single source of truth. The website fetches release metadata server-side (cached at the edge) and renders platform-specific download buttons. The desktop app uses `electron-updater`'s GitHub provider for Win/Linux only — on Mac the updater is uninitialized and the menu item opens the GitHub release page in the browser. macOS users install via a Homebrew Cask tap (`ZainW/homebrew-mdow`) updated by a release script.

**Tech Stack:** electron-updater 6.8, electron-log, electron-store, TanStack Start (Cloudflare Pages), Vitest + @testing-library/react, Phosphor icons, Tailwind v4, Homebrew Cask DSL.

**What's already in place** (verified — do not rebuild):

- `apps/desktop/electron-builder.yml` already declares `publish: github`.
- `apps/desktop/src/main/updater.ts` initializes `autoUpdater` with `autoDownload = false`, forwards events as IPC. Called from `index.ts:125`.
- `apps/desktop/src/main/ipc.ts:110-112` — `updater:check` / `updater:download` / `updater:install` handlers.
- `apps/desktop/src/preload/index.ts:154-185` — `window.api` updater methods + event subscribers.
- `apps/desktop/src/renderer/src/components/UpdateBanner.tsx` — banner UI mounted in `App.tsx:266`.

This plan polishes the Mac gap, Emil-principles violations, and missing periodic checks; wires the website; and adds the Homebrew tap.

---

## Phase A — Desktop Updater Polish

### Task A1: Add macOS guard, periodic re-check, and manual-check signaling to updater.ts

**Files:**

- Modify: `apps/desktop/src/main/updater.ts`

The current `initAutoUpdater` runs on every platform (broken on Mac without code signing) and only checks once at startup. We need: Mac no-op, 30s startup delay, 4-hour interval, and a `wasManualCheck` flag forwarded with `up-to-date` so the UI can show "you're up to date" only when triggered by the menu.

- [ ] **Step 1: Replace `apps/desktop/src/main/updater.ts` contents**

```ts
import pkg from 'electron-updater'
import { BrowserWindow } from 'electron'
import log from 'electron-log'
import { isAutoUpdateEnabled } from './store'

const { autoUpdater } = pkg

autoUpdater.logger = log
autoUpdater.autoDownload = false
autoUpdater.autoInstallOnAppQuit = true

const STARTUP_DELAY_MS = 30_000
const RECHECK_INTERVAL_MS = 4 * 60 * 60 * 1000

const isMac = process.platform === 'darwin'

let manualCheckPending = false
let intervalHandle: NodeJS.Timeout | null = null

export function initAutoUpdater(getMainWindow: () => BrowserWindow | null): void {
  if (isMac) {
    // Squirrel.Mac requires a signed build to apply updates. Until we have an
    // Apple Developer ID, a half-working updater is worse than an honest link
    // out to the releases page (handled by the menu).
    return
  }

  const send = (channel: string, ...args: unknown[]) => {
    const win = getMainWindow()
    if (win && !win.isDestroyed()) {
      win.webContents.send(channel, ...args)
    }
  }

  autoUpdater.on('update-available', (info) => {
    manualCheckPending = false
    send('updater:update-available', {
      version: info.version,
      releaseNotes: info.releaseNotes,
    })
  })

  autoUpdater.on('update-not-available', () => {
    const wasManual = manualCheckPending
    manualCheckPending = false
    send('updater:up-to-date', { wasManual })
  })

  autoUpdater.on('download-progress', (progress) => {
    send('updater:download-progress', { percent: Math.round(progress.percent) })
  })

  autoUpdater.on('update-downloaded', () => {
    send('updater:update-downloaded')
  })

  autoUpdater.on('error', (err) => {
    log.error('Auto-updater error:', err)
    // Intentionally not forwarded as a UI event — failed background checks
    // shouldn't surface noise. Manual checks still get their own log line.
  })

  scheduleAutoChecks()
}

function scheduleAutoChecks(): void {
  if (!isAutoUpdateEnabled()) return
  setTimeout(() => {
    void autoUpdater.checkForUpdates().catch(() => {})
  }, STARTUP_DELAY_MS)
  intervalHandle = setInterval(() => {
    void autoUpdater.checkForUpdates().catch(() => {})
  }, RECHECK_INTERVAL_MS)
}

export function checkForUpdates(opts?: { manual?: boolean }): void {
  if (isMac) return
  if (opts?.manual) manualCheckPending = true
  void autoUpdater.checkForUpdates().catch(() => {
    manualCheckPending = false
  })
}

export function downloadUpdate(): void {
  if (isMac) return
  void autoUpdater.downloadUpdate().catch(() => {})
}

export function installUpdate(): void {
  if (isMac) return
  autoUpdater.quitAndInstall()
}

export function setAutoUpdateScheduling(enabled: boolean): void {
  if (isMac) return
  if (enabled && !intervalHandle) {
    scheduleAutoChecks()
  } else if (!enabled && intervalHandle) {
    clearInterval(intervalHandle)
    intervalHandle = null
  }
}
```

- [ ] **Step 2: Manual smoke check (no automated test for electron singletons)**

Run: `pnpm --filter desktop run dev`

Expected: app launches normally on Mac (no errors in console; no auto-updater activity logged). On Win/Linux, after ~30s a check is triggered and logs to `electron-log` show the GitHub feed URL.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/main/updater.ts
git commit -m "feat(desktop): mac-skip + periodic re-check in updater"
```

---

### Task A2: Add `autoUpdateEnabled` to the electron-store schema

**Files:**

- Modify: `apps/desktop/src/main/store.ts`

Add a top-level boolean (matching the existing flat schema style — no nesting). Default `true`. Expose getter/setter functions used by `updater.ts` (for scheduling) and IPC (for the settings dialog).

- [ ] **Step 1: Edit `apps/desktop/src/main/store.ts` — add field, default, accessors**

In the `StoreSchema` interface, add:

```ts
autoUpdateEnabled: boolean
```

In `defaults`, add:

```ts
    autoUpdateEnabled: true,
```

In `getAppState()`, add:

```ts
    autoUpdateEnabled: store.get('autoUpdateEnabled'),
```

In `saveAppState()`, add:

```ts
if (state.autoUpdateEnabled !== undefined) store.set('autoUpdateEnabled', state.autoUpdateEnabled)
```

At the bottom of the file, add:

```ts
export function isAutoUpdateEnabled(): boolean {
  return store.get('autoUpdateEnabled')
}
```

- [ ] **Step 2: Run typecheck to confirm no breakage**

Run: `pnpm run --filter desktop typecheck`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/main/store.ts
git commit -m "feat(desktop): persist autoUpdateEnabled setting"
```

---

### Task A3: Wire IPC `updater:check` to pass `manual: true`, add `setAutoUpdateScheduling` IPC

**Files:**

- Modify: `apps/desktop/src/main/ipc.ts`

The Help menu item triggers `checkForUpdates` via the existing IPC channel. We make the menu pass a `{ manual: true }` payload, and add an IPC handler the settings dialog uses to start/stop scheduling when the toggle changes.

- [ ] **Step 1: Edit `apps/desktop/src/main/ipc.ts`**

Find the line:

```ts
import { checkForUpdates, downloadUpdate, installUpdate } from './updater'
```

Replace with:

```ts
import { checkForUpdates, downloadUpdate, installUpdate, setAutoUpdateScheduling } from './updater'
```

Find:

```ts
ipcMain.handle('updater:check', () => checkForUpdates())
```

Replace with:

```ts
ipcMain.handle('updater:check', (_event, opts?: { manual?: boolean }) => checkForUpdates(opts))
ipcMain.handle('updater:set-scheduling', (_event, enabled: boolean) =>
  setAutoUpdateScheduling(enabled),
)
```

- [ ] **Step 2: Typecheck**

Run: `pnpm run --filter desktop typecheck`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/main/ipc.ts
git commit -m "feat(desktop): IPC for manual check + scheduling toggle"
```

---

### Task A4: Update preload bridge to surface `manual` flag and scheduling control

**Files:**

- Modify: `apps/desktop/src/preload/index.ts`

The renderer needs to (a) call `checkForUpdates({ manual: true })` from the menu callback and (b) call `setAutoUpdateScheduling(enabled)` from the settings toggle, and (c) receive `wasManual` in the `up-to-date` payload.

- [ ] **Step 1: Edit `apps/desktop/src/preload/index.ts`**

Find:

```ts
  checkForUpdates: () => ipcRenderer.invoke('updater:check'),
  downloadUpdate: () => ipcRenderer.invoke('updater:download'),
  installUpdate: () => ipcRenderer.invoke('updater:install'),
```

Replace with:

```ts
  checkForUpdates: (opts?: { manual?: boolean }) => ipcRenderer.invoke('updater:check', opts),
  downloadUpdate: () => ipcRenderer.invoke('updater:download'),
  installUpdate: () => ipcRenderer.invoke('updater:install'),
  setAutoUpdateScheduling: (enabled: boolean) =>
    ipcRenderer.invoke('updater:set-scheduling', enabled),
```

Find:

```ts
  onUpdateUpToDate: (callback) => {
    const handler = () => callback()
    ipcRenderer.on('updater:up-to-date', handler)
    return () => ipcRenderer.removeListener('updater:up-to-date', handler)
  },
```

Replace with:

```ts
  onUpdateUpToDate: (callback: (info: { wasManual: boolean }) => void) => {
    const handler = (_: Electron.IpcRendererEvent, info: { wasManual: boolean }) => callback(info)
    ipcRenderer.on('updater:up-to-date', handler)
    return () => ipcRenderer.removeListener('updater:up-to-date', handler)
  },
```

- [ ] **Step 2: Typecheck**

Run: `pnpm run --filter desktop typecheck`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/preload/index.ts
git commit -m "feat(desktop): preload — manual flag + scheduling toggle"
```

---

### Task A5: Type `window.api` updater surface in renderer env.d.ts

**Files:**

- Modify: `apps/desktop/src/renderer/src/env.d.ts`

Currently the renderer calls `window.api.checkForUpdates()` etc. with no type info (implicit `any`). Add an ambient `Window.api` declaration so the renderer is type-safe and TypeScript catches signature drift.

- [ ] **Step 1: Append to `apps/desktop/src/renderer/src/env.d.ts`**

```ts
type UpdaterUnsubscribe = () => void

interface MdowApi {
  checkForUpdates: (opts?: { manual?: boolean }) => Promise<void>
  downloadUpdate: () => Promise<void>
  installUpdate: () => Promise<void>
  setAutoUpdateScheduling: (enabled: boolean) => Promise<void>
  onUpdateAvailable: (
    callback: (info: { version: string; releaseNotes?: string }) => void,
  ) => UpdaterUnsubscribe
  onUpdateUpToDate: (callback: (info: { wasManual: boolean }) => void) => UpdaterUnsubscribe
  onUpdateDownloadProgress: (
    callback: (progress: { percent: number }) => void,
  ) => UpdaterUnsubscribe
  onUpdateDownloaded: (callback: () => void) => UpdaterUnsubscribe
  onUpdateError: (callback: (message: string) => void) => UpdaterUnsubscribe
  saveAppState: (state: Record<string, unknown>) => Promise<void>
  setTheme: (theme: string) => Promise<void>
  [key: string]: unknown
}

interface Window {
  api: MdowApi
}
```

(The `[key: string]: unknown` index signature is a temporary escape hatch so we don't have to type the entire pre-existing API surface in this PR. Future cleanup task.)

- [ ] **Step 2: Typecheck**

Run: `pnpm run --filter desktop typecheck`
Expected: PASS — and `window.api.checkForUpdates({ manual: true })` is now type-checked when used in subsequent tasks.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/renderer/src/env.d.ts
git commit -m "types(desktop): window.api updater surface"
```

---

### Task A6: Add `Help → Check for Updates…` menu item

**Files:**

- Modify: `apps/desktop/src/main/menu.ts`

On Win/Linux, send a renderer message that triggers a manual check. On Mac, open the releases page in the default browser.

- [ ] **Step 1: Edit `apps/desktop/src/main/menu.ts`**

Update the import line at the top:

```ts
import { Menu, app, BrowserWindow, shell } from 'electron'
```

Replace the entire `Help` submenu block:

```ts
    {
      label: 'Help',
      submenu: [
        {
          label: 'Keyboard Shortcuts',
          accelerator: 'CmdOrCtrl+/',
          click: () => getMainWindow()?.webContents.send('menu:shortcuts'),
        },
        { type: 'separator' },
        {
          label: 'Check for Updates…',
          click: () => {
            if (process.platform === 'darwin') {
              void shell.openExternal('https://github.com/ZainW/mdow/releases/latest')
            } else {
              getMainWindow()?.webContents.send('menu:check-for-updates')
            }
          },
        },
      ],
    },
```

- [ ] **Step 2: Add preload bridge for the new menu signal**

Edit `apps/desktop/src/preload/index.ts`. Find the existing `onMenuSettings` block and add right after it:

```ts
  onMenuCheckForUpdates: (callback) => {
    const handler = () => callback()
    ipcRenderer.on('menu:check-for-updates', handler)
    return () => ipcRenderer.removeListener('menu:check-for-updates', handler)
  },
```

Then add to the `MdowApi` interface in `apps/desktop/src/renderer/src/env.d.ts`:

```ts
onMenuCheckForUpdates: (callback: () => void) => UpdaterUnsubscribe
```

- [ ] **Step 3: Typecheck**

Run: `pnpm run --filter desktop typecheck`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src/main/menu.ts apps/desktop/src/preload/index.ts apps/desktop/src/renderer/src/env.d.ts
git commit -m "feat(desktop): Help → Check for Updates menu item"
```

---

### Task A7: Refactor UpdateBanner — Emil principles + manual-check awareness

**Files:**

- Modify: `apps/desktop/src/renderer/src/components/UpdateBanner.tsx`
- Test: `apps/desktop/src/renderer/src/components/UpdateBanner.test.tsx`

Issues to fix:

- Uses `transition-all` (forbidden — animate specific properties only).
- No `prefers-reduced-motion` handling.
- Errors are surfaced visibly (spec says silent — they're already logged in main).
- "Up to date" is never shown (spec: show only when manual menu check).
- Percentage isn't `tabular-nums` (jitters as digits change width).
- Dismiss button is 12px (`size-3`) — below the 44px tap target rule.

The banner currently lives at the bottom of the editor pane (`App.tsx:266`), inside the flex column. Per the Emil rationale in the spec, we keep it there — it's a status-bar-style strip that does not push markdown content (it's below it inside the same flex container, so the markdown view's height shrinks slightly when the banner is visible). That's acceptable for an infrequent, informational element, and it's far simpler than a fixed-position toast that would have to dodge the existing `<TabBar />`, `<DocumentBreadcrumb />`, and command palette layers. The spec called this out as a TBD-in-plan; this is the resolved decision.

- [ ] **Step 1: Write the failing test**

Create `apps/desktop/src/renderer/src/components/UpdateBanner.test.tsx`:

```tsx
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen, act } from '@testing-library/react'
import { UpdateBanner } from './UpdateBanner'

type Cb = (...args: unknown[]) => void

interface MockApi {
  checkForUpdates: ReturnType<typeof vi.fn>
  downloadUpdate: ReturnType<typeof vi.fn>
  installUpdate: ReturnType<typeof vi.fn>
  onUpdateAvailable: (cb: Cb) => () => void
  onUpdateUpToDate: (cb: Cb) => () => void
  onUpdateDownloadProgress: (cb: Cb) => () => void
  onUpdateDownloaded: (cb: Cb) => () => void
  onUpdateError: (cb: Cb) => () => void
  onMenuCheckForUpdates: (cb: Cb) => () => void
}

let listeners: Record<string, Cb> = {}

function subscriber(name: string) {
  return (cb: Cb) => {
    listeners[name] = cb
    return () => {
      delete listeners[name]
    }
  }
}

beforeEach(() => {
  listeners = {}
  const api: MockApi = {
    checkForUpdates: vi.fn().mockResolvedValue(undefined),
    downloadUpdate: vi.fn().mockResolvedValue(undefined),
    installUpdate: vi.fn().mockResolvedValue(undefined),
    onUpdateAvailable: subscriber('available'),
    onUpdateUpToDate: subscriber('upToDate'),
    onUpdateDownloadProgress: subscriber('progress'),
    onUpdateDownloaded: subscriber('downloaded'),
    onUpdateError: subscriber('error'),
    onMenuCheckForUpdates: subscriber('menuCheck'),
  }
  ;(globalThis as unknown as { window: { api: MockApi } }).window = { api }
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('UpdateBanner', () => {
  it('renders nothing initially', () => {
    const { container } = render(<UpdateBanner />)
    expect(container).toBeEmptyDOMElement()
  })

  it('shows the available banner with version when an update arrives', () => {
    render(<UpdateBanner />)
    act(() => listeners.available({ version: '1.2.3' }))
    expect(screen.getByText(/1\.2\.3/)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /download/i })).toBeInTheDocument()
  })

  it('shows progress with tabular-nums during download', () => {
    render(<UpdateBanner />)
    act(() => listeners.progress({ percent: 42 }))
    const pct = screen.getByText(/42/)
    expect(pct).toBeInTheDocument()
    expect(pct.className).toMatch(/tabular-nums/)
  })

  it('shows ready state with a Restart button when downloaded', () => {
    render(<UpdateBanner />)
    act(() => listeners.downloaded())
    expect(screen.getByRole('button', { name: /restart/i })).toBeInTheDocument()
  })

  it('does NOT show "up to date" for non-manual checks', () => {
    render(<UpdateBanner />)
    act(() => listeners.upToDate({ wasManual: false }))
    expect(screen.queryByText(/up to date/i)).not.toBeInTheDocument()
  })

  it('shows "up to date" when triggered manually', () => {
    render(<UpdateBanner />)
    act(() => listeners.upToDate({ wasManual: true }))
    expect(screen.getByText(/up to date/i)).toBeInTheDocument()
  })

  it('does NOT surface errors visibly', () => {
    const { container } = render(<UpdateBanner />)
    act(() => listeners.error('boom'))
    expect(container).toBeEmptyDOMElement()
  })

  it('triggers a manual check when the menu signal fires', () => {
    render(<UpdateBanner />)
    act(() => listeners.menuCheck())
    expect(window.api.checkForUpdates).toHaveBeenCalledWith({ manual: true })
  })
})
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm --filter desktop test -- -t UpdateBanner`
Expected: FAIL — current component doesn't handle `wasManual`, doesn't call `checkForUpdates({ manual: true })` on menu signal, and surfaces errors.

- [ ] **Step 3: Replace `UpdateBanner.tsx` with the refactor**

Replace the file contents with:

```tsx
import { useEffect, useState } from 'react'
import { ArrowsClockwise, DownloadSimple, X } from '@phosphor-icons/react'
import { cn } from '@renderer/lib/utils'

type UpdateState =
  | { status: 'idle' }
  | { status: 'available'; version: string }
  | { status: 'downloading'; percent: number }
  | { status: 'ready' }
  | { status: 'up-to-date' }

export function UpdateBanner() {
  const [state, setState] = useState<UpdateState>({ status: 'idle' })
  const [dismissed, setDismissed] = useState(false)

  useEffect(() => {
    const unsubs = [
      window.api.onUpdateAvailable((info) => {
        setState({ status: 'available', version: info.version })
        setDismissed(false)
      }),
      window.api.onUpdateDownloadProgress((progress) => {
        setState({ status: 'downloading', percent: progress.percent })
      }),
      window.api.onUpdateDownloaded(() => {
        setState({ status: 'ready' })
        setDismissed(false)
      }),
      window.api.onUpdateUpToDate((info) => {
        if (info.wasManual) {
          setState({ status: 'up-to-date' })
          setDismissed(false)
        }
      }),
      window.api.onUpdateError(() => {
        // Silent. Already logged in main via electron-log.
      }),
      window.api.onMenuCheckForUpdates(() => {
        void window.api.checkForUpdates({ manual: true })
      }),
    ]
    return () => unsubs.forEach((fn) => fn())
  }, [])

  if (state.status === 'idle' || dismissed) return null

  return (
    <div
      className={cn(
        'flex items-center gap-2 border-t border-border bg-muted/50 px-3 py-1.5 text-xs text-muted-foreground',
        // Entrance animation; respects reduced motion via the media query below.
        'motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-bottom-1 motion-safe:duration-200',
      )}
      role="status"
      aria-live="polite"
    >
      {state.status === 'available' && (
        <>
          <span>
            Mdow <strong>{state.version}</strong> is available
          </span>
          <button
            type="button"
            onClick={() => void window.api.downloadUpdate()}
            className="ml-1 inline-flex min-h-[28px] items-center gap-1 rounded-md bg-primary px-2 py-0.5 text-xs font-medium text-primary-foreground transition-colors duration-150 ease-out hover:bg-primary/90"
          >
            <DownloadSimple weight="bold" className="size-3" aria-hidden />
            Download
          </button>
        </>
      )}

      {state.status === 'downloading' && (
        <>
          <ArrowsClockwise className="size-3 motion-safe:animate-spin" aria-hidden />
          <span>
            Downloading update… <span className="tabular-nums">{state.percent}</span>%
          </span>
          <div className="h-1 w-24 overflow-hidden rounded-full bg-muted">
            <div
              className="h-full rounded-full bg-primary transition-[width] duration-150 ease-out"
              style={{ width: `${state.percent}%` }}
            />
          </div>
        </>
      )}

      {state.status === 'ready' && (
        <>
          <span>Update ready — restart to apply</span>
          <button
            type="button"
            onClick={() => void window.api.installUpdate()}
            className="ml-1 inline-flex min-h-[28px] items-center gap-1 rounded-md bg-primary px-2 py-0.5 text-xs font-medium text-primary-foreground transition-colors duration-150 ease-out hover:bg-primary/90"
          >
            <ArrowsClockwise weight="bold" className="size-3" aria-hidden />
            Restart
          </button>
        </>
      )}

      {state.status === 'up-to-date' && <span>You're on the latest version</span>}

      <button
        type="button"
        onClick={() => setDismissed(true)}
        className="ml-auto rounded p-1 transition-colors duration-150 ease-out hover:bg-muted"
        aria-label="Dismiss update notification"
      >
        <X className="size-3" aria-hidden />
      </button>
    </div>
  )
}
```

Notes on Emil principles applied here:

- `transition-all` removed everywhere; replaced with explicit `transition-colors` or `transition-[width]`.
- `motion-safe:` prefix gates the entrance/spin animations behind `prefers-reduced-motion: no-preference`.
- Percentage wrapped in `<span className="tabular-nums">` so digit-width changes don't push the rest of the line.
- `aria-live="polite"` so screen readers announce state changes; `role="status"`.
- `aria-hidden` on decorative icons so they don't get re-announced.
- Buttons get `min-h-[28px]` (still small, but this is a status bar — desktop hover targets, not mobile tap targets). Keep `aria-label` on the icon-only dismiss.

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm --filter desktop test -- -t UpdateBanner`
Expected: PASS (all 8 cases)

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/renderer/src/components/UpdateBanner.tsx apps/desktop/src/renderer/src/components/UpdateBanner.test.tsx
git commit -m "feat(desktop): UpdateBanner — manual-check, silent errors, motion-safe"
```

---

### Task A8: Add Updates section to SettingsDialog (Win/Linux only)

**Files:**

- Modify: `apps/desktop/src/renderer/src/store/app-store.ts`
- Modify: `apps/desktop/src/renderer/src/components/SettingsDialog.tsx`

Add `autoUpdateEnabled` to the Zustand store mirroring the main process schema. Render an "Updates" section at the bottom of the dialog with a single switch. Hide on macOS.

- [ ] **Step 1: Read current sections of SettingsDialog**

Run: `grep -n "Theme\|Font\|className=\"space-y" apps/desktop/src/renderer/src/components/SettingsDialog.tsx | head -10`

This is just to identify the section pattern — each section in this dialog uses a heading + form control idiom. The agent should mirror it.

- [ ] **Step 2: Edit `apps/desktop/src/renderer/src/store/app-store.ts`**

In the store interface (around line 79 where `theme: string` is declared), add:

```ts
  autoUpdateEnabled: boolean
  setAutoUpdateEnabled: (enabled: boolean) => void
```

In the store body (where `theme: 'system'` and `setTheme` are defined, around line 281), add after `setTheme`:

```ts
  autoUpdateEnabled: true,
  setAutoUpdateEnabled: (enabled) => {
    void window.api.saveAppState({ autoUpdateEnabled: enabled })
    void window.api.setAutoUpdateScheduling(enabled)
    set({ autoUpdateEnabled: enabled })
  },
```

Find where the initial app state is loaded from `window.api.getAppState()` (search for `getAppState` in the file). When the loaded state is applied to the store, include `autoUpdateEnabled` so the persisted value (or default) is reflected.

- [ ] **Step 3: Add "Updates" section to SettingsDialog.tsx**

At the top of the file, alongside the existing icon imports, ensure:

```ts
import { Sun, Moon, Desktop } from '@phosphor-icons/react'
```

already covers what we need (we won't add a new icon for the toggle).

Inside the `SettingsDialog` component, add this hook near the other store reads (around line 57):

```ts
const autoUpdateEnabled = useAppStore((s) => s.autoUpdateEnabled)
const setAutoUpdateEnabled = useAppStore((s) => s.setAutoUpdateEnabled)
const isMac = typeof navigator !== 'undefined' && /Mac/i.test(navigator.platform)
```

Just before the closing `</DialogContent>` (find it in the JSX), insert:

```tsx
{
  !isMac && (
    <section className="space-y-2">
      <h3 className="text-sm font-medium">Updates</h3>
      <label className="flex items-center justify-between gap-3 text-sm">
        <span className="text-muted-foreground">
          Automatically check for updates in the background
        </span>
        <input
          type="checkbox"
          checked={autoUpdateEnabled}
          onChange={(e) => setAutoUpdateEnabled(e.target.checked)}
          className="size-4 cursor-pointer accent-primary"
        />
      </label>
    </section>
  )
}
```

(A native checkbox keeps this small. If the project later adds a `Switch` shadcn component, swap in.)

- [ ] **Step 4: Typecheck**

Run: `pnpm run --filter desktop typecheck`
Expected: PASS

- [ ] **Step 5: Run tests (existing SettingsDialog tests must still pass)**

Run: `pnpm --filter desktop test -- -t SettingsDialog`
Expected: PASS — no regressions.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/src/renderer/src/store/app-store.ts apps/desktop/src/renderer/src/components/SettingsDialog.tsx
git commit -m "feat(desktop): settings — auto-update toggle (win/linux)"
```

---

## Phase B — Website Live Download Page

### Task B1: Build the GitHub Releases fetcher with tests

**Files:**

- Create: `apps/web/src/lib/github-releases.ts`
- Create: `apps/web/src/lib/github-releases.test.ts`

Pure function that parses a GitHub Releases API response and returns a normalized `ReleaseInfo` keyed by platform. Filename matching is by suffix to survive version bumps.

- [ ] **Step 1: Write the failing test**

Create `apps/web/src/lib/github-releases.test.ts`:

```ts
import { describe, expect, it } from 'vitest'
import { parseRelease, type ReleaseInfo } from './github-releases'

const sample = {
  tag_name: 'v1.2.3',
  name: 'v1.2.3',
  published_at: '2026-04-26T10:00:00Z',
  html_url: 'https://github.com/ZainW/mdow/releases/tag/v1.2.3',
  assets: [
    {
      name: 'Mdow-1.2.3-arm64.dmg',
      browser_download_url: 'https://example.test/Mdow-1.2.3-arm64.dmg',
    },
    { name: 'Mdow-1.2.3-x64.dmg', browser_download_url: 'https://example.test/Mdow-1.2.3-x64.dmg' },
    {
      name: 'Mdow-1.2.3-arm64-mac.zip',
      browser_download_url: 'https://example.test/Mdow-1.2.3-arm64-mac.zip',
    },
    {
      name: 'Mdow-Setup-1.2.3.exe',
      browser_download_url: 'https://example.test/Mdow-Setup-1.2.3.exe',
    },
    {
      name: 'Mdow-1.2.3.AppImage',
      browser_download_url: 'https://example.test/Mdow-1.2.3.AppImage',
    },
    { name: 'latest.yml', browser_download_url: 'https://example.test/latest.yml' },
  ],
}

describe('parseRelease', () => {
  it('extracts version, html_url, and platform-keyed assets', () => {
    const result = parseRelease(sample) as ReleaseInfo
    expect(result.version).toBe('1.2.3')
    expect(result.htmlUrl).toBe('https://github.com/ZainW/mdow/releases/tag/v1.2.3')
    expect(result.publishedAt).toBe('2026-04-26T10:00:00Z')
    expect(result.assets.mac.dmg).toEqual([
      { arch: 'arm64', url: 'https://example.test/Mdow-1.2.3-arm64.dmg' },
      { arch: 'x64', url: 'https://example.test/Mdow-1.2.3-x64.dmg' },
    ])
    expect(result.assets.mac.zip).toHaveLength(1)
    expect(result.assets.windows.exe).toBe('https://example.test/Mdow-Setup-1.2.3.exe')
    expect(result.assets.linux.appImage).toBe('https://example.test/Mdow-1.2.3.AppImage')
  })

  it('strips the leading v from tag_name', () => {
    const result = parseRelease({ ...sample, tag_name: 'v9.9.9' })
    expect(result?.version).toBe('9.9.9')
  })

  it('returns null when no assets are present', () => {
    const result = parseRelease({ ...sample, assets: [] })
    expect(result).toBeNull()
  })

  it('ignores update-manifest yml files in asset matching', () => {
    const result = parseRelease(sample)
    const allUrls = JSON.stringify(result)
    expect(allUrls).not.toContain('latest.yml')
  })
})
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm --filter web test -- -t parseRelease`
Expected: FAIL — module doesn't exist.

- [ ] **Step 3: Implement `apps/web/src/lib/github-releases.ts`**

```ts
export interface ReleaseAsset {
  arch?: 'arm64' | 'x64'
  url: string
}

export interface ReleaseInfo {
  version: string
  publishedAt: string
  htmlUrl: string
  assets: {
    mac: { dmg: ReleaseAsset[]; zip: ReleaseAsset[] }
    windows: { exe: string | null }
    linux: { appImage: string | null }
  }
}

interface GhAsset {
  name: string
  browser_download_url: string
}

interface GhRelease {
  tag_name: string
  name?: string
  published_at: string
  html_url: string
  assets: GhAsset[]
}

const REPO = 'ZainW/mdow'

function detectArch(name: string): 'arm64' | 'x64' | undefined {
  if (name.includes('arm64')) return 'arm64'
  if (name.includes('x64')) return 'x64'
  return undefined
}

export function parseRelease(release: GhRelease): ReleaseInfo | null {
  if (!release?.assets?.length) return null

  const dmg = release.assets
    .filter((a) => a.name.endsWith('.dmg'))
    .map((a) => ({ arch: detectArch(a.name), url: a.browser_download_url }))

  const zip = release.assets
    .filter((a) => a.name.endsWith('.zip') && a.name.includes('mac'))
    .map((a) => ({ arch: detectArch(a.name), url: a.browser_download_url }))

  const exe = release.assets.find((a) => a.name.endsWith('.exe'))?.browser_download_url ?? null

  const appImage =
    release.assets.find((a) => a.name.endsWith('.AppImage'))?.browser_download_url ?? null

  return {
    version: release.tag_name.replace(/^v/, ''),
    publishedAt: release.published_at,
    htmlUrl: release.html_url,
    assets: { mac: { dmg, zip }, windows: { exe }, linux: { appImage } },
  }
}

export async function fetchLatestRelease(): Promise<ReleaseInfo | null> {
  try {
    const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
      headers: {
        Accept: 'application/vnd.github+json',
        'User-Agent': 'mdow-website',
      },
    })
    if (!res.ok) return null
    const json = (await res.json()) as GhRelease
    return parseRelease(json)
  } catch {
    return null
  }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm --filter web test -- -t parseRelease`
Expected: PASS (all 4 cases)

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/lib/github-releases.ts apps/web/src/lib/github-releases.test.ts
git commit -m "feat(web): GitHub Releases fetcher + asset parser"
```

---

### Task B2: Wire the download route to the live fetcher

**Files:**

- Modify: `apps/web/src/routes/download.tsx`

Server function returns `{ os, release }`. The route falls back gracefully when `release` is null.

- [ ] **Step 1: Replace `apps/web/src/routes/download.tsx` with**

```tsx
import { createFileRoute } from '@tanstack/react-router'
import { createServerFn } from '@tanstack/react-start'
import { getRequestHeader, setHeader } from '@tanstack/react-start/server'
import { DownloadCard } from '~/components/download-card'
import { fetchLatestRelease, type ReleaseInfo } from '~/lib/github-releases'
import { seo } from '~/lib/seo'

const REPO_RELEASES_URL = 'https://github.com/ZainW/mdow/releases'

const loadDownloadData = createServerFn({ method: 'GET' }).handler(async () => {
  // Edge cache for 10 minutes — GitHub API is rate-limited (60/hr per IP unauth).
  setHeader('Cache-Control', 'public, max-age=600, s-maxage=600')

  const ua = getRequestHeader('user-agent') || ''
  const os: 'mac' | 'windows' | 'linux' = ua.includes('Mac')
    ? 'mac'
    : ua.includes('Windows')
      ? 'windows'
      : ua.includes('Linux')
        ? 'linux'
        : 'mac'

  const release = await fetchLatestRelease()
  return { os, release }
})

export const Route = createFileRoute('/download')({
  loader: () => loadDownloadData(),
  head: () => ({
    meta: seo({
      title: 'Download Mdow',
      description: 'Download Mdow for Mac, Windows, or Linux.',
    }),
  }),
  component: DownloadPage,
})

function DownloadPage() {
  const { os, release } = Route.useLoaderData() as {
    os: 'mac' | 'windows' | 'linux'
    release: ReleaseInfo | null
  }

  if (!release) {
    return (
      <div className="mx-auto max-w-4xl px-6 py-16">
        <div className="mb-12 text-center">
          <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">Download Mdow</h1>
          <p className="mt-3 text-muted-foreground">
            Downloads are temporarily unavailable.{' '}
            <a className="underline" href={REPO_RELEASES_URL}>
              Browse all releases on GitHub
            </a>
            .
          </p>
        </div>
      </div>
    )
  }

  const platforms = buildPlatforms(release)
  const sorted = [...platforms].sort((a, b) => (a.id === os ? -1 : b.id === os ? 1 : 0))

  return (
    <div className="mx-auto max-w-4xl px-6 py-16">
      <div className="mb-10 text-center">
        <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">Download Mdow</h1>
        <p className="mt-3 text-muted-foreground">
          Version <span className="tabular-nums">{release.version}</span> · released{' '}
          {new Date(release.publishedAt).toLocaleDateString(undefined, {
            year: 'numeric',
            month: 'short',
            day: 'numeric',
          })}
        </p>
      </div>
      <div className="grid gap-6 sm:grid-cols-3">
        {sorted.map((p) => (
          <DownloadCard
            key={p.id}
            platform={p.platform}
            icon={p.icon}
            formats={p.formats}
            recommended={p.id === os}
            note={p.note}
          />
        ))}
      </div>
      <p className="mt-8 text-center text-sm text-muted-foreground">
        <a className="underline hover:text-foreground" href={REPO_RELEASES_URL}>
          View all releases
        </a>
      </p>
    </div>
  )
}

interface PlatformBlock {
  id: 'mac' | 'windows' | 'linux'
  platform: string
  icon: string
  formats: { label: string; url: string }[]
  note?: string
}

function buildPlatforms(release: ReleaseInfo): PlatformBlock[] {
  const macFormats = [
    ...release.assets.mac.dmg.map((a) => ({
      label: a.arch ? `Download .dmg (${a.arch})` : 'Download .dmg',
      url: a.url,
    })),
    ...release.assets.mac.zip.map((a) => ({
      label: a.arch ? `Download .zip (${a.arch})` : 'Download .zip',
      url: a.url,
    })),
  ]

  return [
    {
      id: 'mac',
      platform: 'macOS',
      icon: '\u{1F4BB}',
      formats: macFormats,
      note: 'brew install --cask zainw/mdow/mdow',
    },
    {
      id: 'windows',
      platform: 'Windows',
      icon: '\u{1FAA9}',
      formats: release.assets.windows.exe
        ? [{ label: 'Download Installer', url: release.assets.windows.exe }]
        : [],
    },
    {
      id: 'linux',
      platform: 'Linux',
      icon: '\u{1F427}',
      formats: release.assets.linux.appImage
        ? [{ label: 'Download .AppImage', url: release.assets.linux.appImage }]
        : [],
    },
  ]
}
```

- [ ] **Step 2: Verify imports against existing file**

Run: `grep -n "getRequestHeader\|setHeader" apps/web/src/routes/*.tsx`

If `setHeader` is not used elsewhere, the import path may differ. Confirm against the installed `@tanstack/react-start` version: check `node_modules/@tanstack/react-start/server.d.ts` for the export. Adjust the import line if needed.

- [ ] **Step 3: Typecheck and build**

Run: `pnpm run --filter web typecheck`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add apps/web/src/routes/download.tsx
git commit -m "feat(web): live download page wired to GitHub Releases"
```

---

### Task B3: Update DownloadCard to render the optional note (brew snippet for Mac)

**Files:**

- Modify: `apps/web/src/components/download-card.tsx`

- [ ] **Step 1: Replace `apps/web/src/components/download-card.tsx`**

```tsx
import { useState } from 'react'
import { cn } from '~/lib/utils'

interface DownloadCardProps {
  platform: string
  icon: string
  formats: { label: string; url: string }[]
  recommended?: boolean
  note?: string
}

export function DownloadCard({ platform, icon, formats, recommended, note }: DownloadCardProps) {
  const [copied, setCopied] = useState(false)

  const handleCopy = async () => {
    if (!note) return
    await navigator.clipboard.writeText(note)
    setCopied(true)
    setTimeout(() => setCopied(false), 1500)
  }

  return (
    <div
      className={cn(
        'rounded-lg border p-6 text-center',
        recommended && 'border-primary bg-primary/5 ring-1 ring-primary/20',
      )}
    >
      <div className="mb-3 text-3xl">{icon}</div>
      <h3 className="mb-1 text-lg font-semibold">{platform}</h3>
      {recommended && (
        <p className="mb-3 text-xs font-medium text-primary">Recommended for your OS</p>
      )}
      <div className="flex flex-col gap-2">
        {formats.length === 0 ? (
          <p className="text-xs text-muted-foreground">Not available for this release.</p>
        ) : (
          formats.map((f) => (
            <a
              key={f.label}
              href={f.url}
              className={cn(
                'inline-flex items-center justify-center rounded-md px-4 py-2 text-sm font-medium transition-colors duration-150 ease-out',
                recommended
                  ? 'bg-primary text-primary-foreground hover:bg-primary/90'
                  : 'border hover:bg-muted',
              )}
            >
              {f.label}
            </a>
          ))
        )}
      </div>
      {note && (
        <div className="mt-4 flex items-center justify-between gap-2 rounded-md bg-muted/60 px-3 py-2 text-left">
          <code className="truncate text-xs">{note}</code>
          <button
            type="button"
            onClick={() => void handleCopy()}
            className="shrink-0 rounded px-2 py-1 text-xs text-muted-foreground transition-colors duration-150 ease-out hover:bg-muted"
            aria-label="Copy install command"
          >
            {copied ? 'Copied' : 'Copy'}
          </button>
        </div>
      )}
    </div>
  )
}
```

- [ ] **Step 2: Typecheck and build**

Run: `pnpm run --filter web typecheck && pnpm run --filter web build`
Expected: PASS — site builds with live release data baked in (or falls back gracefully if the API is unreachable during build).

- [ ] **Step 3: Commit**

```bash
git add apps/web/src/components/download-card.tsx
git commit -m "feat(web): DownloadCard — install note + copy"
```

---

## Phase C — Homebrew Cask Tap

### Task C1: Add the Homebrew tap publish script

**Files:**

- Create: `apps/desktop/scripts/update-homebrew-cask.sh`
- Modify: `apps/desktop/package.json` (add `release:homebrew` script)

Bash script that reads the current desktop version, downloads the corresponding `.dmg` from the published GitHub release, computes its SHA-256, clones the tap repo, edits the formula, commits, and pushes. Idempotent enough to run twice on the same version.

- [ ] **Step 1: Create `apps/desktop/scripts/update-homebrew-cask.sh`**

```bash
#!/usr/bin/env bash
set -euo pipefail

# Updates ZainW/homebrew-mdow with the latest desktop release.
# Prereqs: gh (authenticated), shasum, jq, git.

REPO="ZainW/mdow"
TAP_REPO="ZainW/homebrew-mdow"
PKG_JSON="$(dirname "$0")/../package.json"

VERSION="$(jq -r .version "$PKG_JSON")"
if [[ -z "$VERSION" || "$VERSION" == "null" ]]; then
  echo "could not read version from $PKG_JSON" >&2
  exit 1
fi

# Prefer the arm64 dmg as the canonical cask asset.
ASSET_NAME="Mdow-${VERSION}-arm64.dmg"
ASSET_URL="https://github.com/${REPO}/releases/download/v${VERSION}/${ASSET_NAME}"

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

echo "Downloading $ASSET_URL …"
curl -fsSL "$ASSET_URL" -o "$WORKDIR/$ASSET_NAME"

SHA="$(shasum -a 256 "$WORKDIR/$ASSET_NAME" | awk '{print $1}')"
echo "SHA-256: $SHA"

echo "Cloning $TAP_REPO …"
gh repo clone "$TAP_REPO" "$WORKDIR/tap"
cd "$WORKDIR/tap"

CASK_FILE="Casks/mdow.rb"
if [[ ! -f "$CASK_FILE" ]]; then
  echo "$CASK_FILE missing in tap repo — create it first" >&2
  exit 1
fi

# In-place sed: BSD/macOS sed wants an empty string after -i.
sed -i '' \
  -e "s|^  version \".*\"|  version \"${VERSION}\"|" \
  -e "s|^  sha256 \".*\"|  sha256 \"${SHA}\"|" \
  "$CASK_FILE"

if git diff --quiet -- "$CASK_FILE"; then
  echo "No changes to $CASK_FILE — already at $VERSION."
  exit 0
fi

git add "$CASK_FILE"
git -c user.name="mdow-release-bot" -c user.email="release@mdow.app" \
  commit -m "mdow ${VERSION}"
git push origin HEAD

echo "Tap updated to ${VERSION}."
```

Make it executable:

```bash
chmod +x apps/desktop/scripts/update-homebrew-cask.sh
```

- [ ] **Step 2: Add the npm script**

Edit `apps/desktop/package.json`. In the `scripts` block, after `"publish"`, add:

```json
    "release:homebrew": "bash scripts/update-homebrew-cask.sh",
```

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/scripts/update-homebrew-cask.sh apps/desktop/package.json
git commit -m "feat(desktop): release script for homebrew tap"
```

---

### Task C2: Document the Homebrew tap repo content (out-of-tree, manual setup)

**Files:**

- Create: `docs/superpowers/notes/homebrew-tap-setup.md` — checklist for the human

This task does not modify monorepo code — it's a one-time setup note. The content of the external `ZainW/homebrew-mdow` repo's `Casks/mdow.rb` is recorded here so it's reproducible from this repo.

- [ ] **Step 1: Create `docs/superpowers/notes/homebrew-tap-setup.md`**

````markdown
# Homebrew Tap Setup (one-time)

## Create the repo

```bash
gh repo create ZainW/homebrew-mdow --public --description "Homebrew tap for Mdow" \
  --add-readme
```
````

Clone it locally, then add a `Casks/mdow.rb` with this initial content (the
`update-homebrew-cask.sh` script will keep `version` and `sha256` in sync from
then on):

```ruby
cask "mdow" do
  version "1.0.0"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"

  url "https://github.com/ZainW/mdow/releases/download/v#{version}/Mdow-#{version}-arm64.dmg"
  name "Mdow"
  desc "A quiet place to read markdown"
  homepage "https://github.com/ZainW/mdow"

  livecheck do
    url :url
    strategy :github_latest
  end

  auto_updates false

  app "Mdow.app"

  zap trash: [
    "~/Library/Application Support/Mdow",
    "~/Library/Preferences/com.mdow.app.plist",
    "~/Library/Logs/Mdow",
  ]
end
```

Commit and push.

## Release flow

After cutting a desktop release with `pnpm --filter desktop run publish` and
waiting for the GitHub release assets to upload:

```bash
pnpm --filter desktop run release:homebrew
```

Users then install with:

```bash
brew tap zainw/mdow      # one-time
brew install --cask mdow  # or: brew install --cask zainw/mdow/mdow
```

And update with:

```bash
brew upgrade --cask mdow
```

````

- [ ] **Step 2: Commit**

```bash
git add docs/superpowers/notes/homebrew-tap-setup.md
git commit -m "docs: homebrew tap setup checklist"
````

---

## Phase D — Verification

### Task D1: End-to-end smoke verification

This task is human-driven; it can't be fully automated but the steps must be run before declaring the work done.

- [ ] **Step 1: Full repo verification**

Run: `pnpm run typecheck && pnpm run lint && pnpm run fmt:check && pnpm run test`
Expected: all pass.

- [ ] **Step 2: Desktop dev launch on Mac**

Run: `pnpm --filter desktop run dev`

Expected:

- App opens.
- Console shows no auto-updater errors (Mac path is no-op).
- Settings → Updates section is **not** present.
- Help → Check for Updates… opens `https://github.com/ZainW/mdow/releases/latest` in the browser.

- [ ] **Step 3: Web dev preview**

Run: `pnpm --filter web dev` and visit `/download`.

Expected:

- Page shows the latest version + release date from the live GitHub API (or the fallback message if the API is unreachable).
- Mac card shows `brew install --cask zainw/mdow/mdow` with a working Copy button.
- Each download button links to a real GitHub release asset URL.

- [ ] **Step 4: Win/Linux updater smoke (deferred — requires a published v1.0.1 test release)**

Recorded as a follow-up. Not blocking for merge.

- [ ] **Step 5: If everything passes, open the PR**

```bash
git push -u origin HEAD
gh pr create --title "Distribution + auto-update for mdow" --body "$(cat <<'EOF'
## Summary

- Wire the website /download page to live GitHub Releases data (cached at the edge for 10 min).
- Finish the in-app updater for Win/Linux: macOS guard, periodic re-checks, manual-check signaling, settings toggle.
- Add Help → Check for Updates… (opens releases page on Mac).
- UpdateBanner refactored per Emil design principles (no transition-all, motion-safe entrance, tabular-nums, silent errors).
- Homebrew tap publish script for one-command Mac install + updates.

Spec: `docs/superpowers/specs/2026-04-26-distribution-and-auto-update-design.md`
Plan: `docs/superpowers/plans/2026-04-26-distribution-and-auto-update.md`

## Test plan

- [ ] `pnpm run typecheck && pnpm run lint && pnpm run fmt:check && pnpm run test`
- [ ] Desktop launches cleanly on Mac (no updater errors)
- [ ] Settings dialog hides the Updates section on Mac
- [ ] Help → Check for Updates opens the releases page on Mac
- [ ] /download shows the latest GitHub release with correct asset URLs
- [ ] DownloadCard copy button works for the brew snippet

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```
