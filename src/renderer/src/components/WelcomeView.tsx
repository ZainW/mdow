import { useCallback, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore } from '../store/app-store'
import { cn } from '../lib/utils'
import { Button } from './ui/button'
import { Card, CardContent } from './ui/card'
import { FileIcon, FolderOpenIcon } from 'lucide-react'

export function WelcomeView() {
  const openTab = useAppStore((s) => s.openTab)
  const setOpenFolder = useAppStore((s) => s.setOpenFolder)
  const queryClient = useQueryClient()
  const [isDragOver, setIsDragOver] = useState(false)

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

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault()
      setIsDragOver(false)

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

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    setIsDragOver(true)
  }, [])

  const handleDragLeave = useCallback(() => {
    setIsDragOver(false)
  }, [])

  return (
    <div
      className="flex flex-1 flex-col items-center justify-center gap-4 text-muted-foreground"
      onDrop={handleDrop}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
    >
      <Card
        className={cn(
          'border-2 border-dashed text-center transition-[border-color,background-color] duration-150 ease-out',
          isDragOver && 'border-primary bg-primary/5',
        )}
      >
        <CardContent className="flex flex-col items-center gap-2 px-12 py-8">
          <h2 className="text-xl font-semibold text-foreground">mdview</h2>
          <p className="text-sm">Drop a markdown file here to view it</p>
        </CardContent>
      </Card>
      <div className="flex gap-2">
        <Button variant="outline" onClick={() => void handleOpenFile()}>
          <FileIcon data-icon="inline-start" />
          Open File
        </Button>
        <Button variant="outline" onClick={() => void handleOpenFolder()}>
          <FolderOpenIcon data-icon="inline-start" />
          Open Folder
        </Button>
      </div>
    </div>
  )
}
