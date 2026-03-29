import { useCallback, useEffect } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore } from './store/app-store'
import { useTheme } from './hooks/useTheme'
import { useFolderTree } from './hooks/useFolderTree'
import { Sidebar } from './components/Sidebar'
import { MarkdownView } from './components/MarkdownView'
import { WelcomeView } from './components/WelcomeView'
import { CommandPalette } from './components/CommandPalette'
import { SidebarProvider } from './components/ui/sidebar'

function App(): React.JSX.Element {
  const activeFile = useAppStore((s) => s.activeFile)
  const sidebarOpen = useAppStore((s) => s.sidebarOpen)
  const toggleSidebar = useAppStore((s) => s.toggleSidebar)
  const setActiveFile = useAppStore((s) => s.setActiveFile)
  const setOpenFolder = useAppStore((s) => s.setOpenFolder)
  const setSidebarWidth = useAppStore((s) => s.setSidebarWidth)
  const openFolderPath = useAppStore((s) => s.openFolderPath)
  const setCommandPaletteOpen = useAppStore((s) => s.setCommandPaletteOpen)
  const updateActiveFileContent = useAppStore((s) => s.updateActiveFileContent)
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
      window.api.onMenuOpenFile(async () => {
        const result = await window.api.openFileDialog()
        if (result) {
          setActiveFile(result)
          void queryClient.invalidateQueries({ queryKey: ['recents'] })
        }
      }),
      window.api.onMenuOpenFolder(async () => {
        const result = await window.api.openFolderDialog()
        if (result) {
          setOpenFolder(result.path, result.tree)
        }
      }),
      window.api.onFileOpened((file) => {
        setActiveFile(file)
        void queryClient.invalidateQueries({ queryKey: ['recents'] })
      }),
      window.api.onFileChanged((content) => {
        updateActiveFileContent(content)
      }),
    ]
    return () => unsubs.forEach((fn) => fn())
  }, [setActiveFile, setOpenFolder, updateActiveFileContent, queryClient])

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
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [setCommandPaletteOpen, toggleSidebar])

  const handleDrop = useCallback(
    async (e: React.DragEvent) => {
      e.preventDefault()
      const files = Array.from(e.dataTransfer.files)
      const mdFile = files.find(
        (f) => f.name.endsWith('.md') || f.name.endsWith('.markdown') || f.name.endsWith('.mdx')
      )
      if (mdFile) {
        const content = await window.api.readFile(mdFile.path)
        setActiveFile({ path: mdFile.path, content })
        void queryClient.invalidateQueries({ queryKey: ['recents'] })
      }
    },
    [setActiveFile, queryClient]
  )

  return (
    <SidebarProvider>
      <div
        className="flex h-screen w-screen overflow-hidden bg-background text-foreground"
        onDrop={handleDrop}
        onDragOver={(e) => e.preventDefault()}
      >
        {sidebarOpen && <Sidebar />}
        {activeFile ? (
          <MarkdownView content={activeFile.content} />
        ) : (
          <WelcomeView />
        )}
        <CommandPalette />
      </div>
    </SidebarProvider>
  )
}

export default App
