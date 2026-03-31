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
import { MagnifyingGlass, File, FolderOpen, SidebarSimple } from '@phosphor-icons/react'

export function Sidebar() {
  const sidebarWidth = useAppStore((s) => s.sidebarWidth)
  const setSidebarWidth = useAppStore((s) => s.setSidebarWidth)
  const sidebarOpen = useAppStore((s) => s.sidebarOpen)
  const toggleSidebar = useAppStore((s) => s.toggleSidebar)
  const setCommandPaletteOpen = useAppStore((s) => s.setCommandPaletteOpen)
  const openTab = useAppStore((s) => s.openTab)
  const setOpenFolder = useAppStore((s) => s.setOpenFolder)
  const queryClient = useQueryClient()
  const resizing = useRef(false)

  const handleOpenFile = useCallback(async () => {
    const result = await window.api.openFileDialog()
    if (result) {
      openTab(result)
      void queryClient.invalidateQueries({ queryKey: ['recents'] })
    }
  }, [openTab, queryClient])

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
      if (e.target instanceof HTMLElement) {
        e.target.setPointerCapture(e.pointerId)
      }
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

  const collapsedWidth = 38

  return (
    <>
      <div
        className="shrink-0 overflow-hidden border-r border-border/60"
        style={{
          width: sidebarOpen ? sidebarWidth : collapsedWidth,
          transition: 'width 200ms cubic-bezier(0.23, 1, 0.32, 1)',
        }}
      >
        <ShadcnSidebar
          collapsible="none"
          className="h-full border-none"
          style={{ '--sidebar-width': `${sidebarWidth}px` }}
        >
          <SidebarHeader className="p-2">
            <div className="flex items-center gap-1">
              <Button
                variant="ghost"
                size="icon"
                className="h-7 w-7 shrink-0 text-muted-foreground"
                onClick={toggleSidebar}
                title={sidebarOpen ? 'Collapse sidebar' : 'Expand sidebar'}
              >
                <SidebarSimple className="size-4" />
              </Button>
              <Button
                variant="ghost"
                className="flex-1 justify-start gap-2 text-muted-foreground text-[11px] h-7 min-w-0"
                style={{
                  opacity: sidebarOpen ? 1 : 0,
                  transform: sidebarOpen ? 'translateX(0)' : 'translateX(-4px)',
                  transition: sidebarOpen
                    ? 'opacity 200ms cubic-bezier(0.23, 1, 0.32, 1) 60ms, transform 200ms cubic-bezier(0.23, 1, 0.32, 1) 60ms'
                    : 'opacity 120ms ease, transform 120ms ease',
                  pointerEvents: sidebarOpen ? 'auto' : 'none',
                }}
                onClick={() => setCommandPaletteOpen(true)}
              >
                <MagnifyingGlass />
                <span>Quick Open</span>
                <span className="ml-auto opacity-30 text-[10px] font-mono">{modKey}K</span>
              </Button>
            </div>
          </SidebarHeader>

          <SidebarContent
            style={{
              opacity: sidebarOpen ? 1 : 0,
              transform: sidebarOpen ? 'translateX(0)' : 'translateX(-8px)',
              transition: sidebarOpen
                ? 'opacity 200ms cubic-bezier(0.23, 1, 0.32, 1) 40ms, transform 200ms cubic-bezier(0.23, 1, 0.32, 1) 40ms'
                : 'opacity 100ms ease, transform 100ms ease',
              pointerEvents: sidebarOpen ? 'auto' : 'none',
            }}
          >
            <RecentsList />
            <FolderTree />
          </SidebarContent>

          <SidebarFooter
            className="flex-row gap-1.5 p-2"
            style={{
              opacity: sidebarOpen ? 1 : 0,
              transform: sidebarOpen ? 'translateX(0)' : 'translateX(-4px)',
              transition: sidebarOpen
                ? 'opacity 200ms cubic-bezier(0.23, 1, 0.32, 1) 80ms, transform 200ms cubic-bezier(0.23, 1, 0.32, 1) 80ms'
                : 'opacity 80ms ease, transform 80ms ease',
              pointerEvents: sidebarOpen ? 'auto' : 'none',
            }}
          >
            <Button
              variant="ghost"
              size="sm"
              className="flex-1 text-[11px] text-muted-foreground h-7"
              onClick={() => void handleOpenFile()}
            >
              <File data-icon="inline-start" />
              Open File
            </Button>
            <Button
              variant="ghost"
              size="sm"
              className="flex-1 text-[11px] text-muted-foreground h-7"
              onClick={() => void handleOpenFolder()}
            >
              <FolderOpen data-icon="inline-start" />
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
