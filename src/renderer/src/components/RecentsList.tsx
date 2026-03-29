import { useCallback } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useRecents } from '../hooks/useRecents'
import { useAppStore } from '../store/app-store'
import { basename } from '../lib/path-utils'

export function RecentsList() {
  const { data: recents = [] } = useRecents()
  const setActiveFile = useAppStore((s) => s.setActiveFile)
  const activeFile = useAppStore((s) => s.activeFile)
  const queryClient = useQueryClient()

  const handleClick = useCallback(
    async (path: string) => {
      const content = await window.api.readFile(path)
      setActiveFile({ path, content })
      queryClient.invalidateQueries({ queryKey: ['recents'] })
    },
    [setActiveFile, queryClient]
  )

  const handleContextMenu = useCallback((path: string) => {
    window.api.showInFolder(path)
  }, [])

  if (recents.length === 0) {
    return null
  }

  return (
    <>
      <div className="sidebar-section-label">Recents</div>
      <div className="sidebar-list">
        {recents.map((path) => (
          <div
            key={path}
            className={`sidebar-item ${activeFile?.path === path ? 'active' : ''}`}
            onClick={() => handleClick(path)}
            onContextMenu={() => handleContextMenu(path)}
            title={path}
          >
            {basename(path)}
          </div>
        ))}
      </div>
    </>
  )
}
