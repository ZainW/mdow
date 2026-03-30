import { create } from 'zustand'

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

interface AppStore {
  tabs: Tab[]
  activeTabId: string | null
  openTab: (file: { path: string; content: string }) => void
  closeTab: (tabId: string) => void
  setActiveTab: (tabId: string) => void
  updateTabContent: (path: string, content: string) => void
  updateTabScroll: (tabId: string, scrollPosition: number) => void
  setTabError: (path: string, error: FileError) => void
  clearTabError: (tabId: string) => void

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

  commandPaletteOpen: boolean
  setCommandPaletteOpen: (open: boolean) => void

  searchOpen: boolean
  setSearchOpen: (open: boolean) => void
}

export const selectActiveTab = (s: AppStore): Tab | null =>
  s.tabs.find((t) => t.id === s.activeTabId) ?? null

export const useAppStore = create<AppStore>((set) => ({
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

  commandPaletteOpen: false,
  setCommandPaletteOpen: (open) => set({ commandPaletteOpen: open }),

  searchOpen: false,
  setSearchOpen: (open) => set({ searchOpen: open }),
}))
