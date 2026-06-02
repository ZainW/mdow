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
        const orderedTabs = activePath
          ? [
              ...state.sessionTabs.filter((t) => t.path === activePath),
              ...state.sessionTabs.filter((t) => t.path !== activePath),
            ]
          : state.sessionTabs

        const failedPaths: string[] = []
        for (const tab of orderedTabs) {
          try {
            const content = await window.api.readFile(tab.path)
            openTab({ path: tab.path, content })
          } catch {
            failedPaths.push(tab.path)
          }
        }
        if (failedPaths.length > 0) {
          console.warn(`Failed to restore ${failedPaths.length} session tab(s):`, failedPaths)
        }

        if (state.sessionActiveTabPath) {
          const tabs = useAppStore.getState().tabs
          const active = tabs.find((t) => t.path === state.sessionActiveTabPath)
          if (active) {
            useAppStore.setState({ activeTabId: active.id })
          }
        }
      }

      useAppStore.setState({ initialized: true })
    })
  }, [setOpenFolder, openTab, openErrorTab])
}
