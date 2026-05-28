import { useCallback, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore } from '../store/app-store'
import { useOpenMarkdownFile } from '../hooks/useOpenMarkdownFile'
import { useRecents } from '../hooks/useRecents'
import { basename, isMarkdownPath } from '../lib/path-utils'
import { cn } from '../lib/utils'
import { Button } from './ui/button'
import { Card, CardContent } from './ui/card'
import { Logo } from './Logo'
import { File, FileText, FolderOpen, FlaskConical } from 'lucide-react'
import { openDevWorkspace } from '../dev/open-dev-workspace'

const isDev = import.meta.env.DEV

export function WelcomeView() {
  const openTab = useAppStore((s) => s.openTab)
  const setOpenFolder = useAppStore((s) => s.setOpenFolder)
  const openMarkdownFile = useOpenMarkdownFile()
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
      setOpenFolder(result.path, result.tree, result.truncated)
    }
  }, [setOpenFolder])

  const handleOpenRecent = useCallback(
    async (path: string) => {
      await openMarkdownFile(path)
    },
    [openMarkdownFile],
  )

  const handleDrop = useCallback(
    async (e: React.DragEvent) => {
      e.preventDefault()
      setIsDragOver(false)

      const files = Array.from(e.dataTransfer.files)
      if (files.length === 0) return

      const paths = files.map((f) => window.api.getPathForFile(f))
      const mdPaths = paths.filter((p) => isMarkdownPath(p))

      if (mdPaths.length > 0) {
        await Promise.all(mdPaths.map((path) => openMarkdownFile(path)))
        return
      }

      if (files.length === 1) {
        const folderPath = paths[0]
        try {
          const scan = await window.api.readFolderTree(folderPath)
          setOpenFolder(folderPath, scan.tree, scan.truncated)
        } catch {
          // Not a directory — ignore.
        }
      }
    },
    [openMarkdownFile, setOpenFolder],
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
      onDrop={(e) => void handleDrop(e)}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
    >
      <div
        className={cn(
          'grid w-full max-w-3xl gap-10 px-10',
          recents.length > 0 ? 'grid-cols-[21fr_19fr]' : 'grid-cols-1 justify-items-center',
        )}
      >
        <div
          className={cn('flex flex-col gap-3', recents.length === 0 && 'items-center text-center')}
        >
          <Logo
            className={cn(
              'h-12 w-12 rounded-[22%] shadow-sm ring-1 ring-border/40',
              recents.length === 0 && 'mb-1',
            )}
          />
          <h2 className="text-2xl font-semibold tracking-tight text-foreground">Mdow</h2>
          <p className="max-w-[40ch] text-pretty text-sm/relaxed text-muted-foreground">
            A quiet markdown viewer. Drop a file anywhere, or open one below.
          </p>
          <div className="mt-2 flex flex-wrap justify-center gap-2">
            <Button variant="outline" size="sm" onClick={() => void handleOpenFile()}>
              <File data-icon="inline-start" />
              Open File
            </Button>
            <Button variant="outline" size="sm" onClick={() => void handleOpenFolder()}>
              <FolderOpen data-icon="inline-start" />
              Open Folder
            </Button>
            {isDev && (
              <Button variant="secondary" size="sm" onClick={openDevWorkspace}>
                <FlaskConical data-icon="inline-start" />
                Dev samples
              </Button>
            )}
          </div>
          <Card
            size="sm"
            className={cn(
              'mt-3 w-full max-w-md ring-1 transition-colors duration-150',
              isDragOver
                ? 'bg-primary/5 ring-primary/30'
                : 'ring-dashed ring-border/70 bg-muted/20',
            )}
          >
            <CardContent className="py-3 text-xs text-muted-foreground">
              <strong className="font-medium text-foreground/90">Anywhere in this window</strong>
              {' — drop '}
              <code className="rounded-sm bg-muted px-1 py-px font-mono text-[0.6875rem] text-foreground/80">
                .md
              </code>{' '}
              files or a folder.
            </CardContent>
          </Card>
        </div>
        {recents.length > 0 && (
          <div className="flex flex-col gap-2">
            <p className="font-mono text-[0.6875rem] tracking-[0.18em] text-muted-foreground/70 uppercase">
              Recent
            </p>
            <ul className="flex flex-col gap-px">
              {recents.slice(0, 6).map((path) => (
                <li key={path}>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => void handleOpenRecent(path)}
                    title={path}
                    className="h-8 w-full justify-start gap-2 px-2 font-normal"
                  >
                    <FileText className="size-3.5 shrink-0 text-muted-foreground/60" />
                    <span className="truncate">{basename(path)}</span>
                  </Button>
                </li>
              ))}
            </ul>
          </div>
        )}
      </div>
    </div>
  )
}
