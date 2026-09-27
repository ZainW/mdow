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

      const failedPaths: string[] = []
      const restoreTab = async (path: string, activate: boolean): Promise<boolean> => {
        try {
          const content = await window.api.readFile(path)
          openTab({ path, content }, { activate })
          return true
        } catch {
          failedPaths.push(path)
          return false
        }
      }

      // Files the OS asked us to open (Finder, `open -a`, argv) win focus over the saved session.
      const [launchFile, ...otherLaunchFiles] = state.launchFiles ?? []
      if (launchFile) {
        try {
          const content = await window.api.readFile(launchFile)
          openTab({ path: launchFile, content })
        } catch (err) {
          openErrorTab(launchFile, { type: getReadErrorType(err), path: launchFile })
        }
        useAppStore.setState({ initialized: true })

        void (async () => {
          for (const path of [
            ...(state.sessionTabs ?? []).map((tab) => tab.path),
            ...otherLaunchFiles,
          ]) {
            if (path === launchFile) continue
            // oxlint-disable-next-line no-await-in-loop -- restore background tabs sequentially to avoid an I/O burst at startup.
            await restoreTab(path, false)
          }
          if (failedPaths.length > 0) {
            console.warn(`Failed to restore ${failedPaths.length} tab(s):`, failedPaths)
          }
        })()
        return
      }

      if (state.sessionTabs?.length) {
        const activePath = state.sessionActiveTabPath
        const activeTab = activePath
          ? state.sessionTabs.find((tab) => tab.path === activePath)
          : state.sessionTabs[0]
        const inactiveTabs = activeTab
          ? state.sessionTabs.filter((tab) => tab.path !== activeTab.path)
          : state.sessionTabs

        const restoredActive = activeTab ? await restoreTab(activeTab.path, true) : false
        useAppStore.setState({ initialized: true })
        const restoredActiveId = useAppStore.getState().activeTabId

        void (async () => {
          let hasActiveTab = restoredActive
          let fallbackActivePath = restoredActive ? activeTab?.path : null

          for (const tab of inactiveTabs) {
            // oxlint-disable-next-line no-await-in-loop -- restore background tabs sequentially to avoid an I/O burst at startup.
            const restored = await restoreTab(tab.path, !hasActiveTab)
            if (restored && !hasActiveTab) {
              hasActiveTab = true
              fallbackActivePath = tab.path
            }
          }

          // Only settle the active tab if the reader has not moved on (e.g. opened a file from
          // Finder, or switched tabs) while the background tabs were loading.
          const { tabs, activeTabId } = useAppStore.getState()
          const targetActivePath = restoredActive ? activeTab?.path : fallbackActivePath
          const untouched = activeTabId === restoredActiveId || activeTabId === null
          if (targetActivePath && untouched) {
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
