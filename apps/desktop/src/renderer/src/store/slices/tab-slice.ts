import type { StateCreator } from 'zustand'
import type { RenderResult } from '../../lib/markdown'
import type { PaneId } from '../../../../shared/types'

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
  splitView: boolean
  activePane: PaneId
  primaryPaneTabId: string | null
  secondaryPaneTabId: string | null
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
  setActivePane: (pane: PaneId) => void
  setPaneTab: (pane: PaneId, tabId: string) => void
  enableSplitView: () => void
  disableSplitView: () => void
  toggleSplitView: () => void
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

function findTabIndex(tabs: Tab[], tabId: string | null): number {
  if (!tabId) return -1
  return tabs.findIndex((tab) => tab.id === tabId)
}

function nextDifferentTabId(tabs: Tab[], tabId: string | null): string | null {
  if (tabs.length === 0) return null
  if (!tabId) return tabs[0].id
  const index = findTabIndex(tabs, tabId)
  if (index === -1) return tabs[0].id
  for (let offset = 1; offset < tabs.length; offset++) {
    const next = tabs[(index + offset) % tabs.length]
    if (next.id !== tabId) return next.id
  }
  return null
}

function existingTabId(tabs: Tab[], tabId: string | null): string | null {
  return tabId && tabs.some((tab) => tab.id === tabId) ? tabId : null
}

export const createTabSlice: StateCreator<TabSlice, [], [], TabSlice> = (set) => ({
  tabs: [],
  activeTabId: null,
  splitView: false,
  activePane: 'primary',
  primaryPaneTabId: null,
  secondaryPaneTabId: null,
  openingPath: null,
  renderCache: new Map(),

  openTab: (file, options) =>
    set((state) => {
      const activate = options?.activate ?? true
      const existing = state.tabs.find((t) => t.path === file.path)
      if (existing) {
        const panePatch =
          activate && state.splitView
            ? state.activePane === 'primary'
              ? { primaryPaneTabId: existing.id }
              : { secondaryPaneTabId: existing.id }
            : {}
        return {
          activeTabId: activate ? existing.id : state.activeTabId,
          ...panePatch,
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
      const panePatch =
        activate && state.splitView
          ? state.activePane === 'primary'
            ? { primaryPaneTabId: newTab.id }
            : { secondaryPaneTabId: newTab.id }
          : activate
            ? { primaryPaneTabId: newTab.id }
            : {}
      return { tabs, activeTabId: activate ? newTab.id : state.activeTabId, ...panePatch }
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
      return {
        tabs,
        activeTabId: newTab.id,
        primaryPaneTabId:
          state.splitView && state.activePane === 'secondary' ? state.primaryPaneTabId : newTab.id,
        secondaryPaneTabId:
          state.splitView && state.activePane === 'secondary'
            ? newTab.id
            : state.secondaryPaneTabId,
      }
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
      let primaryPaneTabId = existingTabId(tabs, state.primaryPaneTabId)
      let secondaryPaneTabId = existingTabId(tabs, state.secondaryPaneTabId)
      if (state.primaryPaneTabId === tabId) {
        primaryPaneTabId = activeTabId ?? tabs[0]?.id ?? null
      }
      if (state.secondaryPaneTabId === tabId) {
        secondaryPaneTabId = nextDifferentTabId(tabs, primaryPaneTabId)
      }
      if (primaryPaneTabId && primaryPaneTabId === secondaryPaneTabId) {
        secondaryPaneTabId = nextDifferentTabId(tabs, primaryPaneTabId)
      }
      const splitView = Boolean(
        state.splitView &&
        primaryPaneTabId &&
        secondaryPaneTabId &&
        primaryPaneTabId !== secondaryPaneTabId,
      )
      const activePane: PaneId =
        splitView && activeTabId === secondaryPaneTabId ? 'secondary' : 'primary'
      if (!splitView) {
        secondaryPaneTabId = null
        primaryPaneTabId = activeTabId
      }
      return {
        tabs,
        activeTabId,
        splitView,
        activePane,
        primaryPaneTabId,
        secondaryPaneTabId,
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
        splitView: false,
        activePane: 'primary',
        primaryPaneTabId: tabId,
        secondaryPaneTabId: null,
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
        splitView: false,
        activePane: 'primary',
        primaryPaneTabId: stillActive ? state.activeTabId : tabId,
        secondaryPaneTabId: null,
        renderCache: withoutRenderCache(
          state.renderCache,
          removed.map((t) => t.id),
        ),
      }
    }),

  closeAllTabs: () =>
    set((state) => {
      for (const t of state.tabs) unwatchPath(t.path)
      return {
        tabs: [],
        activeTabId: null,
        splitView: false,
        activePane: 'primary',
        primaryPaneTabId: null,
        secondaryPaneTabId: null,
        renderCache: new Map(),
      }
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

  setActiveTab: (tabId) =>
    set((state) => {
      if (!state.splitView) {
        return { activeTabId: tabId, activePane: 'primary', primaryPaneTabId: tabId }
      }
      if (state.activePane === 'primary') {
        const secondaryPaneTabId =
          state.secondaryPaneTabId === tabId
            ? nextDifferentTabId(state.tabs, tabId)
            : state.secondaryPaneTabId
        return { activeTabId: tabId, primaryPaneTabId: tabId, secondaryPaneTabId }
      }
      const primaryPaneTabId =
        state.primaryPaneTabId === tabId
          ? nextDifferentTabId(state.tabs, tabId)
          : state.primaryPaneTabId
      return { activeTabId: tabId, primaryPaneTabId, secondaryPaneTabId: tabId }
    }),

  setActivePane: (pane) =>
    set((state) => {
      if (!state.splitView && pane === 'secondary') return state
      const tabId = pane === 'primary' ? state.primaryPaneTabId : state.secondaryPaneTabId
      return { activePane: pane, activeTabId: tabId ?? state.activeTabId }
    }),

  setPaneTab: (pane, tabId) =>
    set((state) => {
      if (!state.tabs.some((tab) => tab.id === tabId)) return state
      const currentPrimary =
        existingTabId(state.tabs, state.primaryPaneTabId) ??
        existingTabId(state.tabs, state.activeTabId) ??
        state.tabs[0]?.id ??
        null
      if (pane === 'primary') {
        const secondary =
          existingTabId(state.tabs, state.secondaryPaneTabId) ??
          nextDifferentTabId(state.tabs, tabId)
        return {
          splitView: Boolean(secondary),
          activePane: 'primary',
          primaryPaneTabId: tabId,
          secondaryPaneTabId: secondary,
          activeTabId: tabId,
        }
      }
      return {
        splitView: true,
        activePane: 'secondary',
        primaryPaneTabId:
          currentPrimary === tabId ? nextDifferentTabId(state.tabs, tabId) : currentPrimary,
        secondaryPaneTabId: tabId,
        activeTabId: tabId,
      }
    }),

  enableSplitView: () =>
    set((state) => {
      const primary =
        existingTabId(state.tabs, state.primaryPaneTabId) ??
        existingTabId(state.tabs, state.activeTabId) ??
        state.tabs[0]?.id ??
        null
      const secondary =
        existingTabId(state.tabs, state.secondaryPaneTabId) ??
        nextDifferentTabId(state.tabs, primary)
      return {
        splitView: Boolean(primary),
        activePane: 'primary',
        primaryPaneTabId: primary,
        secondaryPaneTabId: secondary,
        activeTabId: primary,
      }
    }),

  disableSplitView: () =>
    set((state) => {
      const activeTabId =
        existingTabId(state.tabs, state.activeTabId) ??
        existingTabId(state.tabs, state.primaryPaneTabId) ??
        existingTabId(state.tabs, state.secondaryPaneTabId)
      return {
        splitView: false,
        activePane: 'primary',
        primaryPaneTabId: activeTabId,
        secondaryPaneTabId: null,
        activeTabId,
      }
    }),

  toggleSplitView: () =>
    set((state) => {
      if (state.splitView) {
        const activeTabId =
          existingTabId(state.tabs, state.activeTabId) ??
          existingTabId(state.tabs, state.primaryPaneTabId) ??
          existingTabId(state.tabs, state.secondaryPaneTabId)
        return {
          splitView: false,
          activePane: 'primary',
          primaryPaneTabId: activeTabId,
          secondaryPaneTabId: null,
          activeTabId,
        }
      }
      const primary =
        existingTabId(state.tabs, state.primaryPaneTabId) ??
        existingTabId(state.tabs, state.activeTabId) ??
        state.tabs[0]?.id ??
        null
      const secondary =
        existingTabId(state.tabs, state.secondaryPaneTabId) ??
        nextDifferentTabId(state.tabs, primary)
      return {
        splitView: Boolean(primary),
        activePane: 'primary',
        primaryPaneTabId: primary,
        secondaryPaneTabId: secondary,
        activeTabId: primary,
      }
    }),

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
