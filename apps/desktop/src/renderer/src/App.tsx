import { useCallback, useEffect } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore, selectActiveTab } from './store/app-store'
import { useTheme } from './hooks/useTheme'
import { useFolderTree } from './hooks/useFolderTree'
import { Sidebar } from './components/Sidebar'
import { TabBar } from './components/TabBar'
import { MarkdownView } from './components/MarkdownView'
import { WelcomeView } from './components/WelcomeView'
import { ErrorView } from './components/ErrorView'
import { ErrorBoundary } from './components/ErrorBoundary'
import { CommandPalette } from './components/CommandPalette'
import { UpdateBanner } from './components/UpdateBanner'
import { ShortcutsDialog } from './components/ShortcutsDialog'
import { SettingsDialog } from './components/SettingsDialog'
import { SidebarProvider } from './components/ui/sidebar'

function App(): React.JSX.Element {
  const initialized = useAppStore((s) => s.initialized)
  const activeTab = useAppStore(selectActiveTab)
  const toggleSidebar = useAppStore((s) => s.toggleSidebar)
  const openTab = useAppStore((s) => s.openTab)
  const setOpenFolder = useAppStore((s) => s.setOpenFolder)
  const setSidebarWidth = useAppStore((s) => s.setSidebarWidth)
  const openFolderPath = useAppStore((s) => s.openFolderPath)
  const setCommandPaletteOpen = useAppStore((s) => s.setCommandPaletteOpen)
  const setSearchOpen = useAppStore((s) => s.setSearchOpen)
  const updateTabContent = useAppStore((s) => s.updateTabContent)
  const setTabError = useAppStore((s) => s.setTabError)
  const zoomIn = useAppStore((s) => s.zoomIn)
  const zoomOut = useAppStore((s) => s.zoomOut)
  const resetZoom = useAppStore((s) => s.resetZoom)
  const shortcutsDialogOpen = useAppStore((s) => s.shortcutsDialogOpen)
  const setShortcutsDialogOpen = useAppStore((s) => s.setShortcutsDialogOpen)
  const settingsOpen = useAppStore((s) => s.settingsOpen)
  const setSettingsOpen = useAppStore((s) => s.setSettingsOpen)
  const queryClient = useQueryClient()

  useTheme()
  useFolderTree(openFolderPath)

  useEffect(() => {
    void window.api.getAppState().then(async (state) => {
      if (state.sidebarWidth) setSidebarWidth(state.sidebarWidth)
      if (state.zoomLevel && state.zoomLevel !== 100) {
        useAppStore.setState({ zoomLevel: state.zoomLevel })
      }
      if (state.theme) useAppStore.setState({ theme: state.theme })
      // Restore typography settings
      const typo: Record<string, unknown> = {}
      if (state.contentFont) typo.contentFont = state.contentFont
      if (state.codeFont) typo.codeFont = state.codeFont
      if (state.fontSize) typo.fontSize = state.fontSize
      if (state.lineHeight) typo.lineHeight = state.lineHeight
      if (Object.keys(typo).length) useAppStore.setState(typo)
      if (state.lastFolder) {
        void window.api.readFolderTree(state.lastFolder).then((tree) => {
          setOpenFolder(state.lastFolder!, tree)
        })
      }

      // Restore session tabs
      if (state.sessionTabs?.length) {
        for (const tab of state.sessionTabs) {
          try {
            const content = await window.api.readFile(tab.path)
            openTab({ path: tab.path, content })
          } catch {
            // File no longer exists — skip
          }
        }
        // Set the previously active tab
        if (state.sessionActiveTabPath) {
          const tabs = useAppStore.getState().tabs
          const active = tabs.find((t) => t.path === state.sessionActiveTabPath)
          if (active) useAppStore.setState({ activeTabId: active.id })
        }
      }

      useAppStore.setState({ initialized: true })
    })
  }, [setSidebarWidth, setOpenFolder, openTab])

  useEffect(() => {
    const unsubs = [
      window.api.onMenuOpenFile(() => {
        void window.api.openFileDialog().then((result) => {
          if (result) {
            openTab(result)
            void queryClient.invalidateQueries({ queryKey: ['recents'] })
          }
        })
      }),
      window.api.onMenuOpenFolder(() => {
        void window.api.openFolderDialog().then((result) => {
          if (result) {
            setOpenFolder(result.path, result.tree)
          }
        })
      }),
      window.api.onFileOpened((file) => {
        openTab(file)
        void queryClient.invalidateQueries({ queryKey: ['recents'] })
      }),
      window.api.onFileChanged((data) => {
        updateTabContent(data.path, data.content)
      }),
      window.api.onFileDeleted((path) => {
        setTabError(path, { type: 'deleted', path })
      }),
      window.api.onMenuZoomIn(() => zoomIn()),
      window.api.onMenuZoomOut(() => zoomOut()),
      window.api.onMenuZoomReset(() => resetZoom()),
      window.api.onMenuShortcuts(() => setShortcutsDialogOpen(true)),
      window.api.onMenuSettings(() => setSettingsOpen(true)),
    ]
    return () => unsubs.forEach((fn) => fn())
  }, [
    openTab,
    setOpenFolder,
    updateTabContent,
    setTabError,
    queryClient,
    zoomIn,
    zoomOut,
    resetZoom,
    setShortcutsDialogOpen,
    setSettingsOpen,
  ])

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey
      if (mod && e.key === 'k') {
        e.preventDefault()
        setCommandPaletteOpen(true)
      }
      if (mod && e.key === 'b') {
        e.preventDefault()
        toggleSidebar()
      }
      if (mod && e.key === 'f') {
        e.preventDefault()
        setSearchOpen(true)
      }
      if (mod && (e.key === '=' || e.key === '+')) {
        e.preventDefault()
        zoomIn()
      }
      if (mod && e.key === '-') {
        e.preventDefault()
        zoomOut()
      }
      if (mod && e.key === '0') {
        e.preventDefault()
        resetZoom()
      }
      if (mod && e.key === '/') {
        e.preventDefault()
        setShortcutsDialogOpen(true)
      }
      if (mod && e.key === ',') {
        e.preventDefault()
        setSettingsOpen(true)
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [
    setCommandPaletteOpen,
    setSearchOpen,
    toggleSidebar,
    zoomIn,
    zoomOut,
    resetZoom,
    setShortcutsDialogOpen,
    setSettingsOpen,
  ])

  useEffect(() => {
    if (activeTab) {
      const name = activeTab.path.split(/[/\\]/).pop() || 'Mdow'
      void window.api.setWindowTitle(name, activeTab.path)
    } else {
      void window.api.setWindowTitle('Mdow')
    }
  }, [activeTab])

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault()
      const files = Array.from(e.dataTransfer.files)
      const mdFile = files.find(
        (f) => f.name.endsWith('.md') || f.name.endsWith('.markdown') || f.name.endsWith('.mdx'),
      )
      if (mdFile) {
        void window.api.readFile(mdFile.path).then((content) => {
          openTab({ path: mdFile.path, content })
          void queryClient.invalidateQueries({ queryKey: ['recents'] })
        })
      }
    },
    [openTab, queryClient],
  )

  const renderContent = () => {
    if (!activeTab) return <WelcomeView />
    if (activeTab.error) return <ErrorView error={activeTab.error} tabId={activeTab.id} />
    return (
      <ErrorBoundary tabId={activeTab.id}>
        <MarkdownView tab={activeTab} />
      </ErrorBoundary>
    )
  }

  if (!initialized) {
    return <div className="h-screen w-screen bg-background" />
  }

  return (
    <SidebarProvider>
      <div
        className="flex h-screen w-screen overflow-hidden bg-background text-foreground"
        onDrop={handleDrop}
        onDragOver={(e) => e.preventDefault()}
      >
        <Sidebar />
        <div className="flex flex-1 flex-col overflow-hidden">
          <TabBar />
          {renderContent()}
          <UpdateBanner />
        </div>
        <CommandPalette />
        <ShortcutsDialog open={shortcutsDialogOpen} onOpenChange={setShortcutsDialogOpen} />
        <SettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} />
      </div>
    </SidebarProvider>
  )
}

export default App
