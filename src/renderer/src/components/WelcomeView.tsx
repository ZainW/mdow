import { useCallback, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore } from '../store/app-store'

export function WelcomeView() {
  const setActiveFile = useAppStore((s) => s.setActiveFile)
  const setOpenFolder = useAppStore((s) => s.setOpenFolder)
  const queryClient = useQueryClient()
  const [isDragOver, setIsDragOver] = useState(false)

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

  const handleDrop = useCallback(
    async (e: React.DragEvent) => {
      e.preventDefault()
      setIsDragOver(false)

      const files = Array.from(e.dataTransfer.files)
      const mdFile = files.find((f) =>
        f.name.endsWith('.md') || f.name.endsWith('.markdown') || f.name.endsWith('.mdx')
      )

      if (mdFile) {
        const content = await window.api.readFile(mdFile.path)
        setActiveFile({ path: mdFile.path, content })
        queryClient.invalidateQueries({ queryKey: ['recents'] })
      }
    },
    [setActiveFile, queryClient]
  )

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    setIsDragOver(true)
  }, [])

  const handleDragLeave = useCallback(() => {
    setIsDragOver(false)
  }, [])

  return (
    <div
      className="welcome-view"
      onDrop={handleDrop}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
    >
      <div className={`drop-hint ${isDragOver ? 'drag-over' : ''}`}>
        <h2>mdview</h2>
        <p>Drop a markdown file here to view it</p>
      </div>
      <div style={{ display: 'flex', gap: 8 }}>
        <button onClick={handleOpenFile}>Open File</button>
        <button onClick={handleOpenFolder}>Open Folder</button>
      </div>
    </div>
  )
}
