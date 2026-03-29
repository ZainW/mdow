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
    (e: React.MouseEvent) => {
      e.preventDefault()
      resizing.current = true
      const startX = e.clientX
      const startWidth = sidebarWidth

      const handleMouseMove = (e: MouseEvent) => {
        if (!resizing.current) return
        const newWidth = Math.max(200, Math.min(400, startWidth + (e.clientX - startX)))
        setSidebarWidth(newWidth)
      }

      const handleMouseUp = () => {
        resizing.current = false
        void window.api.saveAppState({ sidebarWidth })
        document.removeEventListener('mousemove', handleMouseMove)
        document.removeEventListener('mouseup', handleMouseUp)
      }

      document.addEventListener('mousemove', handleMouseMove)
      document.addEventListener('mouseup', handleMouseUp)
    },
    [sidebarWidth, setSidebarWidth]
  )

  const isMac = navigator.platform.includes('Mac')
  const modKey = isMac ? '\u2318' : 'Ctrl+'

  return (
    <>
      <ShadcnSidebar
        collapsible="none"
        className="border-r"
        style={{ '--sidebar-width': `${sidebarWidth}px` } as React.CSSProperties}
      >
        <SidebarHeader className="p-3">
          <Button
            variant="outline"
            className="w-full justify-start gap-2 text-muted-foreground text-xs"
            onClick={() => setCommandPaletteOpen(true)}
          >
            <SearchIcon />
            <span>Quick Open</span>
            <span className="ml-auto opacity-40 text-[11px]">{modKey}K</span>
          </Button>
        </SidebarHeader>

        <SidebarContent>
          <RecentsList />
          <FolderTree />
        </SidebarContent>

        <SidebarFooter className="flex-row gap-2 p-2">
          <Button variant="outline" size="sm" className="flex-1" onClick={() => void handleOpenFile()}>
            <FileIcon data-icon="inline-start" />
            Open File
          </Button>
          <Button variant="outline" size="sm" className="flex-1" onClick={() => void handleOpenFolder()}>
            <FolderOpenIcon data-icon="inline-start" />
            Open Folder
          </Button>
        </SidebarFooter>
      </ShadcnSidebar>

      <div
        className="w-1 cursor-col-resize shrink-0 hover:bg-primary active:bg-primary"
        role="separator"
        onMouseDown={handleResizeStart}
      />
    </>
  )
}
