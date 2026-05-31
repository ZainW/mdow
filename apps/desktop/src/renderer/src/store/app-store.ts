import { create } from 'zustand'
import {
  createTabSlice,
  type TabSlice,
  type Tab,
  type FileError,
  type ErrorType,
} from './slices/tab-slice'
import { createUiSlice, type UiSlice, type SidebarMode } from './slices/ui-slice'
import { createFolderSlice, type FolderSlice } from './slices/folder-slice'
import { createSettingsSlice, type SettingsSlice } from './slices/settings-slice'

export type { Tab, FileError, ErrorType, SidebarMode }

type AppStore = TabSlice & UiSlice & FolderSlice & SettingsSlice

export const useAppStore = create<AppStore>()((...args) => ({
  ...createTabSlice(...args),
  ...createUiSlice(...args),
  ...createFolderSlice(...args),
  ...createSettingsSlice(...args),
}))

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

function getSessionKey(tabs: Tab[], activeTabId: string | null): string {
  const activeTab = tabs.find((t) => t.id === activeTabId)
  return JSON.stringify({ paths: tabs.map((t) => t.path), activePath: activeTab?.path ?? null })
}

useAppStore.subscribe((state, prev) => {
  if (getSessionKey(state.tabs, state.activeTabId) !== getSessionKey(prev.tabs, prev.activeTabId)) {
    saveSession(state.tabs, state.activeTabId)
  }
})
