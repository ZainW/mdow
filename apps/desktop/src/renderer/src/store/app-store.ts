import { create } from 'zustand'
import type { DocHeading } from '../lib/editor/extensions/heading-ids'

export interface Tab {
  id: string
  path: string
  content: string
  scrollPosition: number
  mode: 'read' | 'edit'
  lastDiskWriteAt?: number
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

interface AppStore {
  initialized: boolean
  tabs: Tab[]
  activeTabId: string | null
  openTab: (file: { path: string; content: string }) => void
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
  toggleTabMode: (tabId: string) => void
  setTabMode: (tabId: string, mode: 'read' | 'edit') => void
  markTabWritten: (path: string, timestamp: number) => void

  sidebarOpen: boolean
  sidebarWidth: number
  toggleSidebar: () => void
  setSidebarWidth: (width: number) => void

  openFolderPath: string | null
  folderTree: TreeNode[]
  setOpenFolder: (path: string, tree: TreeNode[]) => void
  setFolderTree: (tree: TreeNode[]) => void

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

export const useAppStore = create<AppStore>((set) => ({
  initialized: false,
  tabs: [],
  activeTabId: null,

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
        mode: 'read',
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
      return { tabs, activeTabId }
    }),

  closeOtherTabs: (tabId) =>
    set((state) => {
      const keep = state.tabs.find((t) => t.id === tabId)
      if (!keep) return state
      for (const t of state.tabs) {
        if (t.id !== tabId) unwatchPath(t.path)
      }
      return { tabs: [keep], activeTabId: tabId }
    }),

  closeTabsToRight: (tabId) =>
    set((state) => {
      const index = state.tabs.findIndex((t) => t.id === tabId)
      if (index === -1) return state
      const tabs = state.tabs.slice(0, index + 1)
      for (const t of state.tabs.slice(index + 1)) {
        unwatchPath(t.path)
      }
      const stillActive = tabs.some((t) => t.id === state.activeTabId)
      return { tabs, activeTabId: stillActive ? state.activeTabId : tabId }
    }),

  closeAllTabs: () =>
    set((state) => {
      for (const t of state.tabs) unwatchPath(t.path)
      return { tabs: [], activeTabId: null }
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
    set((state) => ({
      tabs: state.tabs.map((t) => (t.path === path ? { ...t, content } : t)),
    })),

  updateTabScroll: (tabId, scrollPosition) =>
    set((state) => ({
      tabs: state.tabs.map((t) => (t.id === tabId ? { ...t, scrollPosition } : t)),
    })),

  setTabError: (path, error) =>
    set((state) => ({
      tabs: state.tabs.map((t) => (t.path === path ? { ...t, error } : t)),
    })),

  clearTabError: (tabId) =>
    set((state) => ({
      tabs: state.tabs.map((t) => (t.id === tabId ? { ...t, error: null } : t)),
    })),

  toggleTabMode: (tabId) =>
    set((state) => ({
      tabs: state.tabs.map((t) =>
        t.id === tabId ? { ...t, mode: t.mode === 'read' ? 'edit' : 'read' } : t,
      ),
    })),

  setTabMode: (tabId, mode) =>
    set((state) => ({
      tabs: state.tabs.map((t) => (t.id === tabId ? { ...t, mode } : t)),
    })),

  markTabWritten: (path, timestamp) =>
    set((state) => ({
      tabs: state.tabs.map((t) => (t.path === path ? { ...t, lastDiskWriteAt: timestamp } : t)),
    })),

  sidebarOpen: true,
  sidebarWidth: 260,
  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
  setSidebarWidth: (width) => set({ sidebarWidth: width }),

  openFolderPath: null,
  folderTree: [],
  setOpenFolder: (path, tree) => set({ openFolderPath: path, folderTree: tree }),
  setFolderTree: (tree) => set({ folderTree: tree }),

  wideMode: false,
  toggleWideMode: () => set((state) => ({ wideMode: !state.wideMode })),

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

// Persist session (open tabs) whenever tabs or active tab change
useAppStore.subscribe((state, prev) => {
  if (state.tabs !== prev.tabs || state.activeTabId !== prev.activeTabId) {
    saveSession(state.tabs, state.activeTabId)
  }
})
