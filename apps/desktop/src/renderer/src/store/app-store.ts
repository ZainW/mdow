import { create } from 'zustand'
import type { DocHeading, RenderResult } from '../lib/markdown'

export interface Tab {
  id: string
  path: string
  content: string
  scrollPosition: number
  error?: FileError | null
}

interface TreeNode {
  name: string
  path: string
  isDirectory: boolean
  children?: TreeNode[]
}

export type ErrorType = 'not-found' | 'permission-denied' | 'deleted' | 'read-error'

export interface FileError {
  type: ErrorType
  path: string
}

export type SidebarMode = 'recents' | 'folder' | 'outline'

interface AppStore {
  initialized: boolean
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

  sidebarOpen: boolean
  sidebarMode: SidebarMode
  toggleSidebar: () => void
  setSidebarMode: (mode: SidebarMode) => void

  openFolderPath: string | null
  folderTree: TreeNode[]
  folderTreeTruncated: boolean
  setOpenFolder: (path: string, tree: TreeNode[], truncated: boolean) => void
  setFolderTree: (tree: TreeNode[], truncated: boolean) => void

  wideMode: boolean
  toggleWideMode: () => void

  docHeadings: DocHeading[]
  activeHeadingId: string | null
  setDocHeadings: (headings: DocHeading[]) => void
  setActiveHeadingId: (id: string | null) => void

  commandPaletteOpen: boolean
  setCommandPaletteOpen: (open: boolean) => void

  searchOpen: boolean
  setSearchOpen: (open: boolean) => void

  shortcutsDialogOpen: boolean
  setShortcutsDialogOpen: (open: boolean) => void

  settingsOpen: boolean
  setSettingsOpen: (open: boolean) => void

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

export const useAppStore = create<AppStore>((set) => ({
  initialized: false,
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
      const newTab: Tab = {
        id: crypto.randomUUID(),
        path,
        content: '',
        scrollPosition: 0,
        error,
      }
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
        if (tabs.length === 0) {
          activeTabId = null
        } else if (index < tabs.length) {
          activeTabId = tabs[index].id
        } else {
          activeTabId = tabs[tabs.length - 1].id
        }
      }
      return {
        tabs,
        activeTabId,
        renderCache: withoutRenderCache(state.renderCache, [tabId]),
      }
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
      for (const t of removed) {
        unwatchPath(t.path)
      }
      const stillActive = tabs.some((t) => t.id === state.activeTabId)
      return {
        tabs,
        activeTabId: stillActive ? state.activeTabId : tabId,
        renderCache: withoutRenderCache(
          state.renderCache,
          removed.map((t) => t.id),
        ),
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

  sidebarOpen: true,
  sidebarMode: 'recents',
  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
  setSidebarMode: (mode) => {
    if (typeof window !== 'undefined' && window.api) {
      void window.api.saveAppState({ sidebarMode: mode })
    }
    set({ sidebarMode: mode })
  },

  openFolderPath: null,
  folderTree: [],
  folderTreeTruncated: false,
  setOpenFolder: (path, tree, truncated) =>
    set({ openFolderPath: path, folderTree: tree, folderTreeTruncated: truncated }),
  setFolderTree: (tree, truncated) => set({ folderTree: tree, folderTreeTruncated: truncated }),

  wideMode: false,
  toggleWideMode: () =>
    set((state) => {
      const wideMode = !state.wideMode
      if (typeof window !== 'undefined' && window.api) {
        void window.api.saveAppState({ wideMode })
      }
      return { wideMode }
    }),

  docHeadings: [],
  activeHeadingId: null,
  setDocHeadings: (headings) => set({ docHeadings: headings }),
  setActiveHeadingId: (id) => set({ activeHeadingId: id }),

  commandPaletteOpen: false,
  setCommandPaletteOpen: (open) => set({ commandPaletteOpen: open }),

  searchOpen: false,
  setSearchOpen: (open) => set({ searchOpen: open }),

  shortcutsDialogOpen: false,
  setShortcutsDialogOpen: (open) => set({ shortcutsDialogOpen: open }),

  settingsOpen: false,
  setSettingsOpen: (open) => set({ settingsOpen: open }),

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
}))

function getSessionKey(tabs: Tab[], activeTabId: string | null): string {
  const activeTab = tabs.find((t) => t.id === activeTabId)
  return JSON.stringify({ paths: tabs.map((t) => t.path), activePath: activeTab?.path ?? null })
}

// Persist session when tab membership/order or active tab changes. Scroll-only tab
// updates are intentionally excluded to avoid writing state during document scroll.
useAppStore.subscribe((state, prev) => {
  if (getSessionKey(state.tabs, state.activeTabId) !== getSessionKey(prev.tabs, prev.activeTabId)) {
    saveSession(state.tabs, state.activeTabId)
  }
})
