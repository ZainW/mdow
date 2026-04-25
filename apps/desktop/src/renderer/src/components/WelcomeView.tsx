import { useCallback, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore } from '../store/app-store'
import { useRecents } from '../hooks/useRecents'
import { basename } from '../lib/path-utils'
import { cn } from '../lib/utils'
import { Button } from './ui/button'
import { File, FileText, FolderOpen } from '@phosphor-icons/react'

export function WelcomeView() {
  const openTab = useAppStore((s) => s.openTab)
  const setOpenFolder = useAppStore((s) => s.setOpenFolder)
  const queryClient = useQueryClient()
  const [isDragOver, setIsDragOver] = useState(false)
  const { data: recents = [] } = useRecents()

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

  const handleOpenRecent = useCallback(
    async (path: string) => {
      const content = await window.api.readFile(path)
      openTab({ path, content })
      void queryClient.invalidateQueries({ queryKey: ['recents'] })
    },
    [openTab, queryClient],
  )

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
      className={cn(
        'flex flex-1 flex-col items-center justify-center text-muted-foreground transition-colors duration-150',
        isDragOver && 'bg-primary/[0.03]',
      )}
      onDrop={handleDrop}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
    >
      <div
        className={cn(
          'grid w-full max-w-3xl gap-10 px-10',
          recents.length > 0 ? 'grid-cols-[21fr_19fr]' : 'grid-cols-1 place-items-center',
        )}
      >
        <div className="flex flex-col gap-3">
          <h2 className="text-2xl font-semibold tracking-tight text-foreground">Mdow</h2>
          <p className="max-w-[40ch] text-pretty text-sm text-muted-foreground">
            A quiet markdown viewer. Drop a file anywhere, or open one below.
          </p>
          <div className="mt-3 flex flex-wrap gap-2">
            <Button variant="outline" onClick={() => void handleOpenFile()}>
              <File data-icon="inline-start" />
              Open File
            </Button>
            <Button variant="outline" onClick={() => void handleOpenFolder()}>
              <FolderOpen data-icon="inline-start" />
              Open Folder
            </Button>
          </div>
          <div
            className={cn(
              'mt-4 rounded-lg border border-dashed px-4 py-3 text-xs transition-colors duration-150 ease-out',
              isDragOver
                ? 'border-primary bg-primary/5 text-foreground'
                : 'border-border-subtle text-muted-foreground',
            )}
          >
            Drop a <span className="font-mono">.md</span> file anywhere on this window.
          </div>
        </div>
        {recents.length > 0 && (
          <div className="flex flex-col gap-2">
            <p className="font-mono text-[0.6875rem] tracking-[0.18em] text-muted-foreground/70 uppercase">
              Recent
            </p>
            <ul role="list" className="flex flex-col gap-px">
              {recents.slice(0, 6).map((path) => (
                <li key={path}>
                  <button
                    type="button"
                    onClick={() => void handleOpenRecent(path)}
                    title={path}
                    className="group flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left transition-colors hover:bg-sidebar-accent"
                  >
                    <FileText className="size-3.5 shrink-0 text-muted-foreground/60 group-hover:text-muted-foreground" />
                    <span className="truncate text-sm text-foreground">{basename(path)}</span>
                  </button>
                </li>
              ))}
            </ul>
          </div>
        )}
      </div>
    </div>
  )
}
