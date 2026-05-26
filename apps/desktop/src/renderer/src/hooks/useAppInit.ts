import { useEffect } from 'react'
import type { AppState } from '../../../shared/types'
import { useAppStore } from '../store/app-store'

export function useAppInit(): void {
  const openTab = useAppStore((s) => s.openTab)
  const setOpenFolder = useAppStore((s) => s.setOpenFolder)

  useEffect(() => {
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
      if (state.fontSize) patch.fontSize = state.fontSize
      if (state.lineHeight) patch.lineHeight = state.lineHeight

      if (Object.keys(patch).length > 0) {
        useAppStore.setState(patch)
      }

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

        for (const tab of orderedTabs) {
          try {
            const content = await window.api.readFile(tab.path)
            openTab({ path: tab.path, content })
          } catch {
            // Skip unreadable session tabs
          }
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
  }, [setOpenFolder, openTab])
}
