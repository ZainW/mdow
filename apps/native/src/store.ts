import { existsSync, realpathSync, statSync } from 'node:fs'
import { resolve } from 'node:path'
import { createStore } from 'zustand/vanilla'
import { useStore } from 'zustand'
import { isSupportedDocument } from './lib/documents'
import { loadDocument, type LoadedDocument } from './lib/document-loader'
import { pushRecent, type PersistedState, type Session } from './lib/persist'
import { clampZoom, ZOOM_STEP, type Prefs } from './lib/prefs'
import { scanWorkspace, type ScanResult } from './lib/workspace'
import type { ColorScheme } from './lib/theme'

export interface Tab {
  path: string
  document: LoadedDocument
  /** The last reload failed; `document` still holds the last good version. */
  reloadFailed: boolean
  /** Bumped on every successful reload so the reader can keep its scroll anchor. */
  revision: number
}

export type Overlay = 'find' | 'palette' | 'settings' | 'shortcuts' | null

export type UpdateStatus =
  | { state: 'idle' }
  | { state: 'checking'; manual: boolean }
  | { state: 'available'; version: string }
  | { state: 'downloading'; version: string }
  | { state: 'ready'; version: string }
  | { state: 'up-to-date'; manual: boolean }
  | { state: 'failed'; manual: boolean; stage: 'check' | 'install'; message: string }
  | { state: 'unavailable'; manual: boolean }

export interface Workspace {
  folder: string
  scan: ScanResult
  collapsed: ReadonlySet<string>
}

export interface AppState {
  prefs: Prefs
  recents: string[]
  tabs: Tab[]
  activePath: string | null
  sidebarOpen: boolean
  workspace: Workspace | null
  overlay: Overlay
  systemScheme: ColorScheme
  update: UpdateStatus
  updateDismissed: boolean
  /** A file drag is over the window (macOS only; gpuix reports just the drop). */
  dragging: boolean
  window: Session['window']
}

export function createAppStore(initial: PersistedState, systemScheme: ColorScheme) {
  const store = createStore<AppState>(() => ({
    prefs: initial.prefs,
    recents: initial.session.recents,
    tabs: [],
    activePath: null,
    sidebarOpen: true,
    workspace: null,
    overlay: null,
    systemScheme,
    update: { state: 'idle' },
    updateDismissed: false,
    dragging: false,
    window: initial.session.window,
  }))
  return store
}

export type AppStore = ReturnType<typeof createAppStore>

let current: AppStore | null = null

export function setAppStore(store: AppStore) {
  current = store
}

export function appStore(): AppStore {
  if (!current) throw new Error('app store not initialized')
  return current
}

export function useApp<T>(selector: (state: AppState) => T): T {
  return useStore(appStore(), selector)
}

const get = () => appStore().getState()
const set = (partial: Partial<AppState> | ((state: AppState) => Partial<AppState>)) =>
  appStore().setState(partial)

// ---------------------------------------------------------------------------
// Documents and tabs
// ---------------------------------------------------------------------------

function canonical(path: string) {
  const absolute = resolve(path)
  try {
    return realpathSync(absolute)
  } catch {
    return absolute
  }
}

/** Open a document, or switch to its tab when it is already open. */
export function openDocument(path: string) {
  const target = canonical(path)
  const existing = get().tabs.find((tab) => tab.path === target)
  if (existing) {
    set({ activePath: target, overlay: null })
    return
  }
  const document = loadDocument(target)
  const tab: Tab = { path: target, document, reloadFailed: false, revision: 0 }
  set((state) => {
    const index = state.tabs.findIndex((item) => item.path === state.activePath)
    const tabs = [...state.tabs]
    tabs.splice(index === -1 ? tabs.length : index + 1, 0, tab)
    return {
      tabs,
      activePath: target,
      overlay: null,
      recents: document.ok ? pushRecent(state.recents, target) : state.recents,
    }
  })
}

/** Folders open as the workspace; supported files open as tabs. */
export function openPaths(paths: string[]) {
  for (const path of paths) {
    let isDir = false
    try {
      isDir = statSync(path).isDirectory()
    } catch {}
    if (isDir) openFolder(path)
    else openDocument(path)
  }
}

export function restoreTabs(paths: string[], active: string | null) {
  const tabs: Tab[] = paths
    .filter((path) => existsSync(path) && isSupportedDocument(path))
    .map((path) => ({ path, document: loadDocument(path), reloadFailed: false, revision: 0 }))
  const activePath = tabs.some((tab) => tab.path === active) ? active : (tabs[0]?.path ?? null)
  set({ tabs, activePath })
}

export function closeTab(path = get().activePath) {
  if (!path) return
  set((state) => {
    const index = state.tabs.findIndex((tab) => tab.path === path)
    if (index === -1) return {}
    const tabs = state.tabs.filter((tab) => tab.path !== path)
    const activePath =
      state.activePath === path
        ? (tabs[Math.min(index, tabs.length - 1)]?.path ?? null)
        : state.activePath
    return { tabs, activePath }
  })
}

export function activateTab(path: string) {
  set({ activePath: path })
}

export function cycleTab(delta: number) {
  const { tabs, activePath } = get()
  if (tabs.length < 2) return
  const index = tabs.findIndex((tab) => tab.path === activePath)
  const next = tabs[(index + delta + tabs.length) % tabs.length]
  if (next) set({ activePath: next.path })
}

export function reloadDocument(path: string) {
  const tab = get().tabs.find((item) => item.path === path)
  if (!tab) return
  const next = loadDocument(path)
  set((state) => ({
    tabs: state.tabs.map((item) => {
      if (item.path !== path) return item
      if (next.ok)
        return { ...item, document: next, reloadFailed: false, revision: item.revision + 1 }
      // Keep the last good render and flag it, rather than swapping in an error screen.
      if (item.document.ok) return { ...item, reloadFailed: true }
      return { ...item, document: next }
    }),
  }))
}

export function forgetRecent(path: string) {
  set((state) => ({ recents: state.recents.filter((item) => item !== path) }))
}

// ---------------------------------------------------------------------------
// Workspace
// ---------------------------------------------------------------------------

export function openFolder(folder: string, reveal = true) {
  const scan = scanWorkspace(folder)
  const path = scan.ok ? scan.root.path : resolve(folder)
  set((state) => ({
    workspace: { folder: path, scan, collapsed: new Set() },
    sidebarOpen: reveal ? true : state.sidebarOpen,
    prefs: reveal ? { ...state.prefs, sidebarMode: 'folder' } : state.prefs,
  }))
}

export function rescanWorkspace() {
  const workspace = get().workspace
  if (!workspace) return
  set({ workspace: { ...workspace, scan: scanWorkspace(workspace.folder) } })
}

export function toggleDirectory(path: string) {
  const workspace = get().workspace
  if (!workspace) return
  const collapsed = new Set(workspace.collapsed)
  if (collapsed.has(path)) collapsed.delete(path)
  else collapsed.add(path)
  set({ workspace: { ...workspace, collapsed } })
}

// ---------------------------------------------------------------------------
// Preferences and chrome
// ---------------------------------------------------------------------------

export function setPrefs(patch: Partial<Prefs>) {
  set((state) => ({ prefs: { ...state.prefs, ...patch } }))
}

export function zoomBy(steps: number) {
  setPrefs({ zoomLevel: clampZoom(get().prefs.zoomLevel + steps * ZOOM_STEP) })
}

export function toggleSidebar() {
  set((state) => ({ sidebarOpen: !state.sidebarOpen }))
}

export function showSidebar(mode: Prefs['sidebarMode']) {
  set((state) => {
    const same = state.sidebarOpen && state.prefs.sidebarMode === mode
    return { sidebarOpen: !same, prefs: { ...state.prefs, sidebarMode: mode } }
  })
}

export function setOverlay(overlay: Overlay) {
  set({ overlay })
}

export function toggleOverlay(overlay: Exclude<Overlay, null>) {
  set((state) => ({ overlay: state.overlay === overlay ? null : overlay }))
}

export function setWindowSize(width: number, height: number) {
  const window = get().window
  if (window?.width === width && window?.height === height) return
  set({ window: { width: Math.round(width), height: Math.round(height) } })
}

export function setDragging(dragging: boolean) {
  if (get().dragging !== dragging) set({ dragging })
}

export function setSystemScheme(systemScheme: ColorScheme) {
  if (get().systemScheme !== systemScheme) set({ systemScheme })
}

export function setUpdate(update: UpdateStatus) {
  set({ update, updateDismissed: false })
}

export function dismissUpdate() {
  set({ updateDismissed: true })
}

export function persistedState(state: AppState): PersistedState {
  return {
    prefs: state.prefs,
    session: {
      recents: state.recents,
      lastFolder: state.workspace?.folder ?? null,
      tabs: state.tabs.map((tab) => tab.path),
      activeTab: state.activePath,
      window: state.window,
    },
  }
}

export function activeScheme(state: Pick<AppState, 'prefs' | 'systemScheme'>): ColorScheme {
  return state.prefs.theme === 'system' ? state.systemScheme : state.prefs.theme
}
