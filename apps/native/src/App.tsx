import { useEffect } from 'react'
import { useGpuixRequired, useWindowSize } from '@gpuix/react'
import { followLink, promptOpenFile, promptOpenFolder, runCommand, setRenderer } from './actions'
import { DOCUMENT_ERROR_COPY } from './lib/documents'
import { readerMaxWidth } from './lib/prefs'
import { openPaths, setDragging, setWindowSize, useApp } from './store'
import { Breadcrumb, ReloadBanner, TabBar, Titlebar } from './ui/Chrome'
import { METRICS, UI_FONT, UiProvider, useUi } from './ui/context'
import { CommandPalette, Settings, Shortcuts } from './ui/Overlays'
import { Button, EmptyState } from './ui/primitives'
import { Reader } from './ui/Reader'
import { Sidebar } from './ui/Sidebar'
import { UpdateBanner } from './ui/UpdateBanner'
import { Welcome } from './ui/Welcome'

export interface AppProps {
  /** Flush persisted state before the process exits for an update restart. */
  saveNow: () => void
}

export function App(props: AppProps) {
  return (
    <UiProvider>
      <Shell {...props} />
    </UiProvider>
  )
}

function Shell({ saveNow }: AppProps) {
  const renderer = useGpuixRequired()
  const { theme } = useUi()
  const size = useWindowSize()
  const sidebarOpen = useApp((state) => state.sidebarOpen)
  const overlay = useApp((state) => state.overlay)
  const prefs = useApp((state) => state.prefs)
  const tab = useApp((state) => state.tabs.find((item) => item.path === state.activePath) ?? null)

  useEffect(() => {
    if (size.width > 0 && size.height > 0) setWindowSize(size.width, size.height)
  }, [size.width, size.height])

  useEffect(() => {
    setRenderer(renderer)
    return () => setRenderer(null)
  }, [renderer])

  useEffect(() => {
    const title = tab?.document.ok ? (tab.document.parsed.title ?? tab.path.split('/').pop()) : null
    renderer.setWindowTitle?.(title ? `${title} — Mdow` : 'Mdow')
  }, [renderer, tab])

  const showSidebar =
    sidebarOpen && size.width >= METRICS.sidebarWidth + METRICS.minMainWidthWithSidebar
  const mainWidth = Math.max(0, size.width - (showSidebar ? METRICS.sidebarWidth : 0))
  const maxWidth = readerMaxWidth(prefs)
  const columnWidth = Math.max(
    0,
    Math.min(maxWidth ?? Infinity, mainWidth - METRICS.readerInset * 2),
  )
  const readerInset = Math.max(METRICS.readerInset, Math.floor((mainWidth - columnWidth) / 2))

  return (
    <div
      onFileDrop={(event) => {
        setDragging(false)
        if (event.paths?.length) openPaths(event.paths)
      }}
      style={{
        display: 'flex',
        flexDirection: 'column',
        width: '100%',
        height: '100%',
        backgroundColor: theme.background,
        color: theme.foreground,
        fontFamily: UI_FONT,
        selectionColor: theme.selection,
      }}
    >
      <Titlebar />
      <div style={{ display: 'flex', flexGrow: 1, minHeight: 0 }}>
        {showSidebar ? <Sidebar onOpenFolder={() => void promptOpenFolder()} /> : null}
        <div style={{ display: 'flex', flexDirection: 'column', flexGrow: 1, minWidth: 0 }}>
          <TabBar />
          <Breadcrumb tab={tab} />
          <UpdateBanner beforeRestart={saveNow} />
          {tab?.reloadFailed ? <ReloadBanner /> : null}
          {!tab ? (
            <Welcome
              onOpenFile={() => void promptOpenFile()}
              onOpenFolder={() => void promptOpenFolder()}
            />
          ) : tab.document.ok ? (
            <Reader
              key={tab.path}
              path={tab.path}
              document={tab.document.parsed}
              columnWidth={columnWidth}
              inset={readerInset}
              onLink={(href) => followLink(href, tab.path)}
            />
          ) : (
            <div
              style={{
                display: 'flex',
                flexGrow: 1,
                alignItems: 'center',
                justifyContent: 'center',
              }}
            >
              <EmptyState
                icon="alert-circle"
                title={DOCUMENT_ERROR_COPY[tab.document.error].title}
                body={DOCUMENT_ERROR_COPY[tab.document.error].body}
                detail={tab.path}
              >
                <Button icon="file" onClick={() => void promptOpenFile()}>
                  Open File
                </Button>
              </EmptyState>
            </div>
          )}
        </div>
      </div>
      {overlay === 'palette' ? <CommandPalette onCommand={runCommand} /> : null}
      {overlay === 'settings' ? (
        <Settings onCheckForUpdates={() => runCommand('check-for-updates')} />
      ) : null}
      {overlay === 'shortcuts' ? <Shortcuts /> : null}
    </div>
  )
}
