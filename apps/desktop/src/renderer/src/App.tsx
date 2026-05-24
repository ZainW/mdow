import { useCallback, useEffect } from 'react'
import { useAppStore, selectActiveTab, type Tab } from './store/app-store'
import { useTheme } from './hooks/useTheme'
import { useFolderTree } from './hooks/useFolderTree'
import { useOpenMarkdownFile } from './hooks/useOpenMarkdownFile'
import { useAppKeyboardShortcuts, useAppMenuBindings } from './hooks/useAppBindings'
import { Sidebar } from './components/Sidebar'
import { TabBar } from './components/TabBar'
import { DocumentBreadcrumb } from './components/DocumentBreadcrumb'
import { MarkdownView } from './components/MarkdownView'
import { WelcomeView } from './components/WelcomeView'
import { ErrorView } from './components/ErrorView'
import { ErrorBoundary } from './components/ErrorBoundary'
import { CommandPalette } from './components/CommandPalette'
import { UpdateBanner } from './components/UpdateBanner'
import { ShortcutsDialog } from './components/ShortcutsDialog'
import { SettingsDialog } from './components/SettingsDialog'
import { SidebarProvider } from './components/ui/sidebar'
import { isTauri } from './lib/is-tauri'
import { basename, isMarkdownPath } from './lib/path-utils'
import { TitlebarInset } from './components/TitlebarInset'
import { IconLab } from './dev/IconLab'

const isIconLab = import.meta.env.VITE_ICON_LAB === 'true'

function App(): React.JSX.Element {
  if (isIconLab) {
    return <IconLab />
  }
  return <MainApp />
}

function MainContent({ activeTab }: { activeTab: Tab | null }): React.JSX.Element {
  if (!activeTab) return <WelcomeView />
  if (activeTab.error) return <ErrorView error={activeTab.error} tabId={activeTab.id} />
  return (
    <ErrorBoundary tabId={activeTab.id}>
      <MarkdownView tab={activeTab} />
    </ErrorBoundary>
  )
}

function MainApp(): React.JSX.Element {
  const initialized = useAppStore((s) => s.initialized)
  const activeTab = useAppStore(selectActiveTab)
  const openTab = useAppStore((s) => s.openTab)
  const setOpenFolder = useAppStore((s) => s.setOpenFolder)
  const openFolderPath = useAppStore((s) => s.openFolderPath)
  const shortcutsDialogOpen = useAppStore((s) => s.shortcutsDialogOpen)
  const setShortcutsDialogOpen = useAppStore((s) => s.setShortcutsDialogOpen)
  const settingsOpen = useAppStore((s) => s.settingsOpen)
  const setSettingsOpen = useAppStore((s) => s.setSettingsOpen)
  const openMarkdownFile = useOpenMarkdownFile()

  useTheme()
  useFolderTree(openFolderPath)
  useAppMenuBindings()
  useAppKeyboardShortcuts()

  useEffect(() => {
    void window.api.getAppState().then(async (state) => {
      const patch: Record<string, unknown> = {}

      if (state.sidebarWidth) patch.sidebarWidth = state.sidebarWidth
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
        void window.api.readFolderTree(state.lastFolder).then((scan) => {
          setOpenFolder(state.lastFolder!, scan.tree, scan.truncated)
        })
      }

      if (state.sessionTabs?.length) {
        const results = await Promise.all(
          state.sessionTabs.map((tab) =>
            window.api
              .readFile(tab.path)
              .then((content) => ({ path: tab.path, content }))
              .catch(() => null),
          ),
        )
        for (const result of results) {
          if (result) openTab(result)
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

  useEffect(() => {
    if (activeTab) {
      void window.api.setWindowTitle(basename(activeTab.path), activeTab.path)
    } else {
      void window.api.setWindowTitle('Mdow')
    }
  }, [activeTab])

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault()
      const files = Array.from(e.dataTransfer.files)
      const mdFile = files.find((f) => isMarkdownPath(f.name))
      if (mdFile) {
        const filePath = window.api.getPathForFile(mdFile)
        void openMarkdownFile(filePath)
      }
    },
    [openMarkdownFile],
  )

  if (!initialized) {
    return (
      <div className="flex h-screen w-screen flex-col bg-background">
        <TitlebarInset />
      </div>
    )
  }

  const htmlDragDropEnabled = !isTauri()

  return (
    <SidebarProvider>
      <div
        className="flex h-screen w-screen flex-col overflow-hidden bg-background text-foreground"
        onDrop={htmlDragDropEnabled ? handleDrop : undefined}
        onDragOver={htmlDragDropEnabled ? (e) => e.preventDefault() : undefined}
      >
        <TitlebarInset />
        <div className="flex min-h-0 flex-1 overflow-hidden">
          <Sidebar />
          <div className="flex flex-1 flex-col overflow-hidden">
            <TabBar />
            {activeTab && <DocumentBreadcrumb tab={activeTab} />}
            <MainContent activeTab={activeTab} />
            <UpdateBanner />
          </div>
          <CommandPalette />
          <ShortcutsDialog open={shortcutsDialogOpen} onOpenChange={setShortcutsDialogOpen} />
          <SettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} />
        </div>
      </div>
    </SidebarProvider>
  )
}

export default App
