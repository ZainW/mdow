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
  openTab: (file: { path: string; content: string }, options?: { activate?: boolean }) => void
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

const MAX_RENDER_CACHE_ENTRIES = 4

export const createTabSlice: StateCreator<TabSlice, [], [], TabSlice> = (set) => ({
  tabs: [],
  activeTabId: null,
  openingPath: null,
  renderCache: new Map(),

  openTab: (file, options) =>
    set((state) => {
      const activate = options?.activate ?? true
      const existing = state.tabs.find((t) => t.path === file.path)
      if (existing) {
        return {
          activeTabId: activate ? existing.id : state.activeTabId,
          renderCache: withoutRenderCache(state.renderCache, [existing.id]),
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
      const insertIndex = activate && activeIndex >= 0 ? activeIndex + 1 : state.tabs.length
      const tabs = [...state.tabs]
      tabs.splice(insertIndex, 0, newTab)
      return { tabs, activeTabId: activate ? newTab.id : state.activeTabId }
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
      renderCache.delete(tabId)
      if (result !== null) {
        renderCache.set(tabId, result)
        while (renderCache.size > MAX_RENDER_CACHE_ENTRIES) {
          const oldest = renderCache.keys().next().value
          if (!oldest) break
          renderCache.delete(oldest)
        }
      }
      return { renderCache }
    }),
})
