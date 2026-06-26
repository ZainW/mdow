import { useEffect } from 'react'
import type { AppState } from '../../../shared/types'
import { getReadErrorType } from '../lib/error-utils'
import { useAppStore } from '../store/app-store'

export function useAppInit(): void {
  const openTab = useAppStore((s) => s.openTab)
  const openErrorTab = useAppStore((s) => s.openErrorTab)
  const setOpenFolder = useAppStore((s) => s.setOpenFolder)

  useEffect(() => {
    const urlParams = new URLSearchParams(window.location.search)
    const openPath = urlParams.get('openPath')

    void window.api.getAppState().then(async (state: AppState) => {
      const patch: Record<string, unknown> = {}

      if (typeof state.wideMode === 'boolean') patch.wideMode = state.wideMode
      if (
        state.sidebarMode === 'recents' ||
        state.sidebarMode === 'folder' ||
        state.sidebarMode === 'outline'
      ) {
        patch.sidebarMode = state.sidebarMode
      }
      if (state.zoomLevel && state.zoomLevel !== 100) patch.zoomLevel = state.zoomLevel
      if (state.theme) patch.theme = state.theme
      if (typeof state.autoUpdateEnabled === 'boolean') {
        patch.autoUpdateEnabled = state.autoUpdateEnabled
      }
      if (state.contentFont) patch.contentFont = state.contentFont
      if (state.codeFont) patch.codeFont = state.codeFont
      if (
        state.interfaceScale === 'compact' ||
        state.interfaceScale === 'comfortable' ||
        state.interfaceScale === 'large'
      ) {
        patch.interfaceScale = state.interfaceScale
      }
      if (
        state.readingWidth === 'standard' ||
        state.readingWidth === 'comfortable' ||
        state.readingWidth === 'wide'
      ) {
        patch.readingWidth = state.readingWidth
      }
      if (
        state.companionProvider === 'auto' ||
        state.companionProvider === 'opencode' ||
        state.companionProvider === 'codex' ||
        state.companionProvider === 'custom'
      ) {
        patch.companionProvider = state.companionProvider
      }
      if (typeof state.companionCustomCommand === 'string') {
        patch.companionCustomCommand = state.companionCustomCommand
      }

      if (Object.keys(patch).length > 0) {
        useAppStore.setState(patch)
      }

      // If openPath query parameter is specified, open it directly
      if (openPath) {
        try {
          const stat = await window.api.statFile(openPath)
          if (stat.exists) {
            if (stat.isDirectory) {
              const scan = await window.api.readFolderTree(openPath)
              setOpenFolder(openPath, scan.tree, scan.truncated)
              useAppStore.setState({ sidebarMode: 'folder' })
            } else if (stat.isFile) {
              const content = await window.api.readFile(openPath)
              openTab({ path: openPath, content })
            }
          }
        } catch (err) {
          openErrorTab(openPath, { type: getReadErrorType(err), path: openPath })
        }
        useAppStore.setState({ initialized: true })
        return
      }

      // Fallback: Standard state restoration
      if (state.lastFolder) {
        void window.api
          .readFolderTree(state.lastFolder)
          .then((scan) => {
            setOpenFolder(state.lastFolder!, scan.tree, scan.truncated)
          })
          .catch(() => {
            void window.api.saveAppState({ lastFolder: null })
          })
      }

      if (state.sessionTabs?.length) {
        const activePath = state.sessionActiveTabPath
        const activeTab = activePath
          ? state.sessionTabs.find((tab) => tab.path === activePath)
          : state.sessionTabs[0]
        const inactiveTabs = activeTab
          ? state.sessionTabs.filter((tab) => tab.path !== activeTab.path)
          : state.sessionTabs

        const failedPaths: string[] = []

        const restoreTab = async (tab: { path: string }, activate: boolean): Promise<boolean> => {
          try {
            const content = await window.api.readFile(tab.path)
            openTab({ path: tab.path, content }, { activate })
            return true
          } catch {
            failedPaths.push(tab.path)
            return false
          }
        }

        const restoredActive = activeTab ? await restoreTab(activeTab, true) : false
        useAppStore.setState({ initialized: true })

        void (async () => {
          let hasActiveTab = restoredActive
          let fallbackActivePath = restoredActive ? activeTab?.path : null

          for (const tab of inactiveTabs) {
            // eslint-disable-next-line no-await-in-loop -- restore background tabs sequentially to avoid an I/O burst at startup.
            const restored = await restoreTab(tab, !hasActiveTab)
            if (restored && !hasActiveTab) {
              hasActiveTab = true
              fallbackActivePath = tab.path
            }
          }

          const targetActivePath = restoredActive ? activeTab?.path : fallbackActivePath
          if (targetActivePath) {
            const tabs = useAppStore.getState().tabs
            const active = tabs.find((t) => t.path === targetActivePath)
            if (active) {
              useAppStore.setState({ activeTabId: active.id })
            }
          }

          if (failedPaths.length > 0) {
            console.warn(`Failed to restore ${failedPaths.length} session tab(s):`, failedPaths)
          }
        })()

        return
      }

      useAppStore.setState({ initialized: true })
    })
  }, [setOpenFolder, openTab, openErrorTab])
}
