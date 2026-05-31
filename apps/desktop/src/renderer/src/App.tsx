import { useCallback, useEffect } from 'react'
import { useAppStore, selectActiveTab, type Tab } from './store/app-store'
import { useTheme } from './hooks/useTheme'
import { useFolderTree } from './hooks/useFolderTree'
import { useOpenMarkdownFile } from './hooks/useOpenMarkdownFile'
import { useAppInit } from './hooks/useAppInit'
import { useAppKeyboardShortcuts, useAppMenuBindings } from './hooks/useAppBindings'
import { Sidebar } from './components/Sidebar'
import { TabBar } from './components/TabBar'
import { DocumentBreadcrumb } from './components/DocumentBreadcrumb'
import { MarkdownView } from './components/MarkdownView'
import { WelcomeView } from './components/WelcomeView'
import { ErrorView } from './components/ErrorView'
import { ErrorBoundary } from './components/ErrorBoundary'
import { AppErrorBoundary } from './components/AppErrorBoundary'
import { CommandPalette } from './components/CommandPalette'
import { UpdateBanner } from './components/UpdateBanner'
import { ShortcutsDialog } from './components/ShortcutsDialog'
import { SettingsDialog } from './components/SettingsDialog'
import { SidebarProvider } from './components/ui/sidebar'
import { basename, isMarkdownPath } from './lib/path-utils'
import { TitlebarInset } from './components/TitlebarInset'
import { Logo } from './components/Logo'
import { IconLab } from './dev/IconLab'

const isIconLab = import.meta.env.VITE_ICON_LAB === 'true'

function App(): React.JSX.Element {
  if (isIconLab) {
    return <IconLab />
  }
  return (
    <AppErrorBoundary>
      <MainApp />
    </AppErrorBoundary>
  )
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

function StartupSplash(): React.JSX.Element {
  return (
    <div className="flex h-screen w-screen flex-col bg-background">
      <TitlebarInset />
      <div className="flex flex-1 flex-col items-center justify-center gap-3 text-muted-foreground">
        <Logo className="h-12 w-12 rounded-[22%] shadow-sm ring-1 ring-border/40" />
        <p className="text-sm text-muted-foreground/80">Loading…</p>
      </div>
    </div>
  )
}

function MainApp(): React.JSX.Element {
  const initialized = useAppStore((s) => s.initialized)
  const activeTab = useAppStore(selectActiveTab)
  const setOpenFolder = useAppStore((s) => s.setOpenFolder)
  const openFolderPath = useAppStore((s) => s.openFolderPath)
  const shortcutsDialogOpen = useAppStore((s) => s.shortcutsDialogOpen)
  const setShortcutsDialogOpen = useAppStore((s) => s.setShortcutsDialogOpen)
  const settingsOpen = useAppStore((s) => s.settingsOpen)
  const setSettingsOpen = useAppStore((s) => s.setSettingsOpen)
  const openMarkdownFile = useOpenMarkdownFile()

  useTheme()
  useAppInit()
  useFolderTree(openFolderPath)
  useAppMenuBindings()
  useAppKeyboardShortcuts()

  useEffect(() => {
    if (activeTab) {
      void window.api.setWindowTitle(basename(activeTab.path), activeTab.path)
    } else {
      void window.api.setWindowTitle('Mdow')
    }
  }, [activeTab])

  const handleDrop = useCallback(
    async (e: React.DragEvent) => {
      e.preventDefault()
      const files = Array.from(e.dataTransfer.files)

      for (const file of files) {
        const filePath = window.api.getPathForFile(file)
        if (isMarkdownPath(file.name)) {
          void openMarkdownFile(filePath)
        }
      }

      if (files.length === 1) {
        const filePath = window.api.getPathForFile(files[0])
        try {
          const stat = await window.api.statFile(filePath)
          if (stat.exists && stat.isDirectory) {
            const result = await window.api.openFolderPath(filePath)
            setOpenFolder(result.path, result.tree, result.truncated)
          }
        } catch {
          // Not a folder or inaccessible
        }
      }
    },
    [openMarkdownFile, setOpenFolder],
  )

  useEffect(() => {
    const handler = (event: Event) => {
      const detail = (event as CustomEvent<{ path: string }>).detail
      if (detail?.path) void openMarkdownFile(detail.path)
    }
    window.addEventListener('mdow:open-markdown-link', handler)
    return () => window.removeEventListener('mdow:open-markdown-link', handler)
  }, [openMarkdownFile])

  if (!initialized) {
    return <StartupSplash />
  }

  return (
    <SidebarProvider>
      <div
        className="flex h-screen w-screen flex-col overflow-hidden bg-background text-foreground"
        onDrop={(e) => void handleDrop(e)}
        onDragOver={(e) => e.preventDefault()}
      >
        <TitlebarInset />
        <div className="flex min-h-0 flex-1 overflow-hidden">
          <Sidebar />
          <main aria-label="Document" className="flex flex-1 flex-col overflow-hidden">
            <TabBar />
            {activeTab && <DocumentBreadcrumb tab={activeTab} />}
            <MainContent activeTab={activeTab} />
            <UpdateBanner />
          </main>
          <CommandPalette />
          <ShortcutsDialog open={shortcutsDialogOpen} onOpenChange={setShortcutsDialogOpen} />
          <SettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} />
        </div>
      </div>
    </SidebarProvider>
  )
}

export default App
