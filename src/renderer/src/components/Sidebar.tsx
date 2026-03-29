import { useCallback, useRef } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore } from '../store/app-store'
import { RecentsList } from './RecentsList'
import { FolderTree } from './FolderTree'

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
      queryClient.invalidateQueries({ queryKey: ['recents'] })
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
        window.api.saveAppState({ sidebarWidth })
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
      <div className="sidebar" style={{ width: sidebarWidth }}>
        <div className="sidebar-header">
          <div className="quick-open-trigger" onClick={() => setCommandPaletteOpen(true)}>
            <span style={{ opacity: 0.5 }}>&#x1F50D;</span>
            <span>Quick Open</span>
            <span className="shortcut">{modKey}K</span>
          </div>
        </div>

        <RecentsList />
        <FolderTree />

        <div className="sidebar-footer">
          <button onClick={handleOpenFile}>Open File</button>
          <button onClick={handleOpenFolder}>Open Folder</button>
        </div>
      </div>
      <div className="resize-handle" onMouseDown={handleResizeStart} />
    </>
  )
}
