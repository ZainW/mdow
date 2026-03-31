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
import { CommandPalette } from './components/CommandPalette'
import { SidebarProvider } from './components/ui/sidebar'

function App(): React.JSX.Element {
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
  const queryClient = useQueryClient()

  useTheme()
  useFolderTree(openFolderPath)

  useEffect(() => {
    void window.api.getAppState().then((state) => {
      if (state.sidebarWidth) setSidebarWidth(state.sidebarWidth)
      if (state.lastFolder) {
        void window.api.readFolderTree(state.lastFolder).then((tree) => {
          setOpenFolder(state.lastFolder!, tree)
        })
      }
    })
  }, [setSidebarWidth, setOpenFolder])

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
    ]
    return () => unsubs.forEach((fn) => fn())
  }, [openTab, setOpenFolder, updateTabContent, setTabError, queryClient])

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
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [setCommandPaletteOpen, setSearchOpen, toggleSidebar])

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
    return <MarkdownView tab={activeTab} />
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
        </div>
        <CommandPalette />
      </div>
    </SidebarProvider>
  )
}

export default App
