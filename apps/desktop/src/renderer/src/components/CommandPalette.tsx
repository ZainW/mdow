import { useCallback, useEffect, useMemo, useState } from 'react'
import { useAppStore } from '../store/app-store'
import { useOpenMarkdownFile } from '../hooks/useOpenMarkdownFile'
import { useRecents } from '../hooks/useRecents'
import { basename, parentDir } from '../lib/path-utils'
import { fuzzySearch } from '../lib/fuzzy-search'
import {
  Command,
  CommandInput,
  CommandList,
  CommandEmpty,
  CommandGroup,
  CommandItem,
} from './ui/command'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from './ui/dialog'
import { FileText } from 'lucide-react'

interface TreeNode {
  name: string
  path: string
  isDirectory: boolean
  children?: TreeNode[]
}

function flattenTree(nodes: TreeNode[], result: { path: string; name: string }[] = []) {
  for (const node of nodes) {
    if (node.isDirectory && node.children) {
      flattenTree(node.children, result)
    } else if (!node.isDirectory) {
      result.push({ path: node.path, name: node.name })
    }
  }
  return result
}

function buildAllFiles(
  folderTree: TreeNode[],
  recents: string[],
): { path: string; name: string }[] {
  const folderFiles = flattenTree(folderTree)
  const folderPaths = new Set(folderFiles.map((f) => f.path))
  const recentFiles: { path: string; name: string }[] = []
  for (const path of recents) {
    if (!folderPaths.has(path)) {
      recentFiles.push({ path, name: basename(path) })
    }
  }
  return [...folderFiles, ...recentFiles]
}

export function CommandPalette() {
  const commandPaletteOpen = useAppStore((s) => s.commandPaletteOpen)
  const setCommandPaletteOpen = useAppStore((s) => s.setCommandPaletteOpen)
  const folderTree = useAppStore((s) => s.folderTree)
  const { data: recents = [] } = useRecents()
  const openMarkdownFile = useOpenMarkdownFile()
  const [query, setQuery] = useState('')
  const allFiles = useMemo(() => {
    if (!commandPaletteOpen) return null
    return buildAllFiles(folderTree, recents)
  }, [commandPaletteOpen, folderTree, recents])

  useEffect(() => {
    if (!commandPaletteOpen) setQuery('')
  }, [commandPaletteOpen])

  const results = useMemo(() => {
    if (!allFiles) return []
    return fuzzySearch(query, allFiles, 50)
  }, [query, allFiles])

  const selectFile = useCallback(
    async (path: string) => {
      setCommandPaletteOpen(false)
      await openMarkdownFile(path)
    },
    [setCommandPaletteOpen, openMarkdownFile],
  )

  return (
    <Dialog open={commandPaletteOpen} onOpenChange={setCommandPaletteOpen}>
      <DialogContent
        className="top-[20%] translate-y-0 overflow-hidden rounded-xl p-0 sm:max-w-lg"
        showCloseButton={false}
        instant
      >
        <DialogHeader className="sr-only">
          <DialogTitle>Quick Open</DialogTitle>
          <DialogDescription>Search for files to open</DialogDescription>
        </DialogHeader>
        <Command shouldFilter={false} value={query} onValueChange={setQuery}>
          <CommandInput placeholder="Search files..." />
          <CommandList>
            <CommandEmpty>No matching files</CommandEmpty>
            <CommandGroup heading="Files">
              {results.map((file) => {
                const dir = parentDir(file.path)
                return (
                  <CommandItem
                    key={file.path}
                    value={file.path}
                    keywords={[file.name]}
                    onSelect={() => void selectFile(file.path)}
                  >
                    <FileText />
                    <span className="min-w-0 truncate">{file.name}</span>
                    {dir && (
                      <span className="ml-auto shrink-0 text-[11px] text-muted-foreground/70">
                        {dir}
                      </span>
                    )}
                  </CommandItem>
                )
              })}
            </CommandGroup>
          </CommandList>
          <div className="flex items-center justify-end gap-3 border-t border-border-subtle bg-muted/40 px-3 py-1.5 text-[10px] text-muted-foreground/80">
            <span className="flex items-center gap-1">
              <kbd className="rounded bg-muted px-1 py-px font-mono text-[10px]">↵</kbd> open
            </span>
            <span className="flex items-center gap-1">
              <kbd className="rounded bg-muted px-1 py-px font-mono text-[10px]">esc</kbd> dismiss
            </span>
          </div>
        </Command>
      </DialogContent>
    </Dialog>
  )
}
