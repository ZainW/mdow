import { lazy, Suspense, useCallback, useEffect } from 'react'
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
import { HtmlView } from './components/HtmlView'
import { WelcomeView } from './components/WelcomeView'
import { ErrorView } from './components/ErrorView'
import { ErrorBoundary } from './components/ErrorBoundary'
import { AppErrorBoundary } from './components/AppErrorBoundary'
import { UpdateBanner } from './components/UpdateBanner'
import { CheatSheetOverlay } from './components/CheatSheetOverlay'
import { DropOverlay } from './components/DropOverlay'
import { useWindowFileDrop } from './hooks/useWindowFileDrop'
import {
  LazyCompanionPanel,
  LazyCompanionShell,
  useCompanionBootstrap,
} from './components/companion/CompanionLazy'
import { DeferredMount } from './components/DeferredMount'
import { SidebarProvider } from './components/ui/sidebar'
import { Button } from './components/ui/button'
import { basename, isDocumentPath, isHtmlPath } from './lib/path-utils'
import { TitlebarInset } from './components/TitlebarInset'
import { Logo } from './components/Logo'
import { FileText, PanelRightOpen } from 'lucide-react'

const isIconLab = import.meta.env.VITE_ICON_LAB === 'true'
const IconLab = lazy(() => import('./dev/IconLab').then((mod) => ({ default: mod.IconLab })))
const CommandPalette = lazy(() =>
  import('./components/CommandPalette').then((mod) => ({ default: mod.CommandPalette })),
)
const SettingsDialog = lazy(() =>
  import('./components/SettingsDialog').then((mod) => ({ default: mod.SettingsDialog })),
)
const ShortcutsDialog = lazy(() =>
  import('./components/ShortcutsDialog').then((mod) => ({ default: mod.ShortcutsDialog })),
)

function App(): React.JSX.Element {
  if (isIconLab) {
    return (
      <Suspense fallback={null}>
        <IconLab />
      </Suspense>
    )
  }
  return (
    <AppErrorBoundary>
      <MainApp />
    </AppErrorBoundary>
  )
}

function DocumentContent({ tab, isActive }: { tab: Tab; isActive: boolean }): React.JSX.Element {
  if (tab.error) return <ErrorView error={tab.error} tabId={tab.id} />
  return (
    <ErrorBoundary tabId={tab.id}>
      {isHtmlPath(tab.path) ? (
        <HtmlView tab={tab} />
      ) : (
        <MarkdownView tab={tab} isActive={isActive} />
      )}
    </ErrorBoundary>
  )
}

function EmptySplitPane({ pane }: { pane: 'primary' | 'secondary' }): React.JSX.Element {
  return (
    <div className="flex flex-1 items-center justify-center px-6 text-center">
      <div className="flex max-w-72 flex-col items-center gap-2 text-sm text-muted-foreground">
        <PanelRightOpen className="size-5 text-muted-foreground/60" aria-hidden />
        <p className="font-medium text-foreground">No document in this pane</p>
        <p className="text-xs leading-5">
          Select this pane, then open a document or choose a tab from the tab bar.
        </p>
        <span className="sr-only">{pane === 'primary' ? 'Left' : 'Right'} pane is empty.</span>
      </div>
    </div>
  )
}

function SplitPane({
  pane,
  tab,
  isActive,
}: {
  pane: 'primary' | 'secondary'
  tab: Tab | null
  isActive: boolean
}): React.JSX.Element {
  const setActivePane = useAppStore((s) => s.setActivePane)
  const disableSplitView = useAppStore((s) => s.disableSplitView)
  const paneLabel = pane === 'primary' ? 'Left' : 'Right'
  const filename = tab ? basename(tab.path) : 'No document'

  return (
    <section
      aria-label={`${paneLabel} document pane`}
      data-active-pane={isActive}
      className="group/pane relative flex min-w-0 flex-1 flex-col overflow-hidden bg-background outline-none data-[active-pane=true]:z-(--z-raised)"
      onPointerDown={() => setActivePane(pane)}
    >
      <div className="flex h-(--breadcrumb-height) shrink-0 items-center gap-2 border-b border-border-subtle bg-background px-3 text-[length:var(--breadcrumb-text-size)]">
        <span
          aria-hidden
          className="h-3.5 w-0.5 rounded-full bg-transparent group-data-[active-pane=true]/pane:bg-primary"
        />
        <FileText className="size-(--button-xs-icon-size) shrink-0 text-muted-foreground/65" />
        <div className="min-w-0 flex-1">
          <p className="truncate font-medium text-foreground/85">{filename}</p>
        </div>
        <span className="shrink-0 rounded-sm bg-muted px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground">
          {paneLabel}
        </span>
        {pane === 'secondary' && (
          <Button
            variant="ghost"
            size="icon-xs"
            aria-label="Close split view"
            title="Close split view"
            onClick={disableSplitView}
            className="text-muted-foreground/70 hover:text-foreground"
          >
            <PanelRightOpen />
          </Button>
        )}
      </div>
      {tab ? <DocumentContent tab={tab} isActive={isActive} /> : <EmptySplitPane pane={pane} />}
    </section>
  )
}

function isOpenDocumentEvent(event: Event): event is CustomEvent<{ path: string }> {
  if (!('detail' in event)) return false
  const detail = event.detail
  return typeof detail === 'object' && detail !== null && 'path' in detail
}

function MainContent({ activeTab }: { activeTab: Tab | null }): React.JSX.Element {
  const tabs = useAppStore((s) => s.tabs)
  const splitView = useAppStore((s) => s.splitView)
  const activePane = useAppStore((s) => s.activePane)
  const primaryPaneTabId = useAppStore((s) => s.primaryPaneTabId)
  const secondaryPaneTabId = useAppStore((s) => s.secondaryPaneTabId)

  if (!activeTab) return <WelcomeView />
  if (!splitView) return <DocumentContent tab={activeTab} isActive />

  const primaryTab = tabs.find((tab) => tab.id === primaryPaneTabId) ?? activeTab
  const secondaryTab = tabs.find((tab) => tab.id === secondaryPaneTabId) ?? null

  return (
    <div className="flex min-h-0 flex-1 overflow-hidden">
      <SplitPane pane="primary" tab={primaryTab} isActive={activePane === 'primary'} />
      <div className="w-px shrink-0 bg-border-subtle" aria-hidden />
      <SplitPane pane="secondary" tab={secondaryTab} isActive={activePane === 'secondary'} />
    </div>
  )
}

function StartupSplash(): React.JSX.Element {
  return (
    <div className="flex h-screen w-screen flex-col bg-background">
      <TitlebarInset />
      <div className="startup-splash flex flex-1 flex-col items-center justify-center gap-3 text-muted-foreground">
        <Logo className="h-12 w-12 rounded-[22%] shadow-sm ring-1 ring-border/40" />
        <p className="text-sm text-muted-foreground/80">Loading…</p>
      </div>
    </div>
  )
}

function MainApp(): React.JSX.Element {
  const initialized = useAppStore((s) => s.initialized)
  const activeTab = useAppStore(selectActiveTab)
  const splitView = useAppStore((s) => s.splitView)
  const setOpenFolder = useAppStore((s) => s.setOpenFolder)
  const openFolderPath = useAppStore((s) => s.openFolderPath)
  const shortcutsDialogOpen = useAppStore((s) => s.shortcutsDialogOpen)
  const setShortcutsDialogOpen = useAppStore((s) => s.setShortcutsDialogOpen)
  const settingsOpen = useAppStore((s) => s.settingsOpen)
  const setSettingsOpen = useAppStore((s) => s.setSettingsOpen)
  const interfaceScale = useAppStore((s) => s.interfaceScale)
  const commandPaletteOpen = useAppStore((s) => s.commandPaletteOpen)
  const openMarkdownFile = useOpenMarkdownFile()

  useTheme()
  useAppInit()
  useFolderTree(openFolderPath)
  useAppMenuBindings()
  useAppKeyboardShortcuts()
  useCompanionBootstrap()

  useEffect(() => {
    document.documentElement.dataset.uiScale = interfaceScale
    return () => {
      delete document.documentElement.dataset.uiScale
    }
  }, [interfaceScale])

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
      const others: string[] = []

      for (const file of files) {
        const filePath = window.api.getPathForFile(file)
        if (isDocumentPath(file.name)) {
          void openMarkdownFile(filePath)
        } else {
          others.push(filePath)
        }
      }

      // Only one folder can be open at a time: open the first one dropped.
      for (const filePath of others) {
        try {
          // oxlint-disable-next-line no-await-in-loop -- stop at the first folder
          const stat = await window.api.statFile(filePath)
          if (stat.exists && stat.isDirectory) {
            // oxlint-disable-next-line no-await-in-loop -- stop at the first folder
            const result = await window.api.openFolderPath(filePath)
            setOpenFolder(result.path, result.tree, result.truncated)
            return
          }
        } catch {
          // Not a folder or inaccessible
        }
      }
    },
    [openMarkdownFile, setOpenFolder],
  )
  const onWindowDrop = useCallback((e: React.DragEvent) => void handleDrop(e), [handleDrop])
  const { summary: dropSummary, handlers: dropHandlers } = useWindowFileDrop(onWindowDrop)

  useEffect(() => {
    const handler = (event: Event) => {
      if (!isOpenDocumentEvent(event)) return
      if (typeof event.detail.path === 'string') void openMarkdownFile(event.detail.path)
    }
    window.addEventListener('mdow:open-document-link', handler)
    return () => window.removeEventListener('mdow:open-document-link', handler)
  }, [openMarkdownFile])

  if (!initialized) {
    return <StartupSplash />
  }

  return (
    <SidebarProvider>
      <div
        data-ui-scale={interfaceScale}
        className="mdow-shell isolate flex h-screen w-screen flex-col overflow-hidden bg-background text-foreground"
        {...dropHandlers}
      >
        <TitlebarInset />
        <div className="flex min-h-0 flex-1 overflow-hidden">
          <LazyCompanionShell>
            <Sidebar />
            <main aria-label="Document" className="flex min-w-0 flex-1 flex-col overflow-hidden">
              <TabBar />
              {activeTab && !splitView && <DocumentBreadcrumb tab={activeTab} />}
              <MainContent activeTab={activeTab} />
              <UpdateBanner />
            </main>
            <LazyCompanionPanel />
          </LazyCompanionShell>
          <DeferredMount when={commandPaletteOpen}>
            <CommandPalette />
          </DeferredMount>
          <CheatSheetOverlay />
          <DropOverlay summary={dropSummary} />
          <DeferredMount when={shortcutsDialogOpen}>
            <ShortcutsDialog open={shortcutsDialogOpen} onOpenChange={setShortcutsDialogOpen} />
          </DeferredMount>
          <DeferredMount when={settingsOpen}>
            <SettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} />
          </DeferredMount>
        </div>
      </div>
    </SidebarProvider>
  )
}

export default App
