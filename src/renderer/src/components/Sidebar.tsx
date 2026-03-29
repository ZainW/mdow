import { useCallback, useRef } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore } from '../store/app-store'
import { RecentsList } from './RecentsList'
import { FolderTree } from './FolderTree'
import { Button } from './ui/button'
import {
  Sidebar as ShadcnSidebar,
  SidebarHeader,
  SidebarContent,
  SidebarFooter,
} from './ui/sidebar'
import { SearchIcon, FileIcon, FolderOpenIcon } from 'lucide-react'

export function Sidebar() {
  const sidebarWidth = useAppStore((s) => s.sidebarWidth)
  const setSidebarWidth = useAppStore((s) => s.setSidebarWidth)
  const sidebarOpen = useAppStore((s) => s.sidebarOpen)
  const setCommandPaletteOpen = useAppStore((s) => s.setCommandPaletteOpen)
  const setActiveFile = useAppStore((s) => s.setActiveFile)
  const setOpenFolder = useAppStore((s) => s.setOpenFolder)
  const queryClient = useQueryClient()
  const resizing = useRef(false)

  const handleOpenFile = useCallback(async () => {
    const result = await window.api.openFileDialog()
    if (result) {
      setActiveFile(result)
      void queryClient.invalidateQueries({ queryKey: ['recents'] })
    }
  }, [setActiveFile, queryClient])

  const handleOpenFolder = useCallback(async () => {
    const result = await window.api.openFolderDialog()
    if (result) {
      setOpenFolder(result.path, result.tree)
    }
  }, [setOpenFolder])

  const handleResizeStart = useCallback(
    (e: React.PointerEvent) => {
      if (!sidebarOpen) return
      e.preventDefault()
      ;(e.target as HTMLElement).setPointerCapture(e.pointerId)
      resizing.current = true
      const startX = e.clientX
      const startWidth = sidebarWidth

      const handlePointerMove = (e: PointerEvent) => {
        if (!resizing.current) return
        const newWidth = Math.max(200, Math.min(400, startWidth + (e.clientX - startX)))
        setSidebarWidth(newWidth)
      }

      const handlePointerUp = () => {
        resizing.current = false
        void window.api.saveAppState({ sidebarWidth })
        document.removeEventListener('pointermove', handlePointerMove)
        document.removeEventListener('pointerup', handlePointerUp)
      }

      document.addEventListener('pointermove', handlePointerMove)
      document.addEventListener('pointerup', handlePointerUp)
    },
    [sidebarWidth, setSidebarWidth, sidebarOpen],
  )

  const isMac = navigator.platform.includes('Mac')
  const modKey = isMac ? '\u2318' : 'Ctrl+'

  return (
    <>
      <div
        className="shrink-0 overflow-hidden border-r border-border/60 transition-[width] duration-150 ease-[cubic-bezier(0.23,1,0.32,1)]"
        style={{ width: sidebarOpen ? sidebarWidth : 0 }}
      >
        <ShadcnSidebar
          collapsible="none"
          className="h-full border-none"
          style={{ '--sidebar-width': `${sidebarWidth}px` } as React.CSSProperties}
        >
          <SidebarHeader className="p-2">
            <Button
              variant="ghost"
              className="w-full justify-start gap-2 text-muted-foreground text-[11px] h-7"
              onClick={() => setCommandPaletteOpen(true)}
            >
              <SearchIcon />
              <span>Quick Open</span>
              <span className="ml-auto opacity-30 text-[10px] font-mono">{modKey}K</span>
            </Button>
          </SidebarHeader>

          <SidebarContent>
            <RecentsList />
            <FolderTree />
          </SidebarContent>

          <SidebarFooter className="flex-row gap-1.5 p-2">
            <Button
              variant="ghost"
              size="sm"
              className="flex-1 text-[11px] text-muted-foreground h-7"
              onClick={() => void handleOpenFile()}
            >
              <FileIcon data-icon="inline-start" />
              Open File
            </Button>
            <Button
              variant="ghost"
              size="sm"
              className="flex-1 text-[11px] text-muted-foreground h-7"
              onClick={() => void handleOpenFolder()}
            >
              <FolderOpenIcon data-icon="inline-start" />
              Open Folder
            </Button>
          </SidebarFooter>
        </ShadcnSidebar>
      </div>

      <div
        className="w-px shrink-0 cursor-col-resize hover:w-0.5 hover:bg-primary/40 active:bg-primary/60 transition-colors"
        role="separator"
        onPointerDown={handleResizeStart}
      />
    </>
  )
}
