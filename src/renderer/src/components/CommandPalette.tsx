import { useCallback, useMemo } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore } from '../store/app-store'
import { useRecents } from '../hooks/useRecents'
import { basename } from '../lib/path-utils'
import {
  CommandDialog,
  CommandInput,
  CommandList,
  CommandEmpty,
  CommandGroup,
  CommandItem,
} from './ui/command'
import { FileIcon } from 'lucide-react'

function flattenTree(nodes: any[], result: { path: string; name: string }[] = []) {
  for (const node of nodes) {
    if (node.isDirectory && node.children) {
      flattenTree(node.children, result)
    } else if (!node.isDirectory) {
      result.push({ path: node.path, name: node.name })
    }
  }
  return result
}

export function CommandPalette() {
  const commandPaletteOpen = useAppStore((s) => s.commandPaletteOpen)
  const setCommandPaletteOpen = useAppStore((s) => s.setCommandPaletteOpen)
  const setActiveFile = useAppStore((s) => s.setActiveFile)
  const folderTree = useAppStore((s) => s.folderTree)
  const { data: recents = [] } = useRecents()
  const queryClient = useQueryClient()

  const allFiles = useMemo(() => {
    const folderFiles = flattenTree(folderTree)
    const folderPaths = new Set(folderFiles.map((f) => f.path))
    const recentFiles = recents
      .filter((path) => !folderPaths.has(path))
      .map((path) => ({ path, name: basename(path) }))
    return [...folderFiles, ...recentFiles]
  }, [folderTree, recents])

  const selectFile = useCallback(
    async (path: string) => {
      setCommandPaletteOpen(false)
      const content = await window.api.readFile(path)
      setActiveFile({ path, content })
      void queryClient.invalidateQueries({ queryKey: ['recents'] })
    },
    [setCommandPaletteOpen, setActiveFile, queryClient]
  )

  return (
    <CommandDialog open={commandPaletteOpen} onOpenChange={setCommandPaletteOpen}>
      <CommandInput placeholder="Search files..." />
      <CommandList>
        <CommandEmpty>No matching files</CommandEmpty>
        <CommandGroup heading="Files">
          {allFiles.map((file) => (
            <CommandItem
              key={file.path}
              value={file.path}
              keywords={[file.name]}
              onSelect={() => void selectFile(file.path)}
            >
              <FileIcon />
              <div className="flex flex-col">
                <span className="text-sm">{file.name}</span>
                <span className="text-xs text-muted-foreground">{file.path}</span>
              </div>
            </CommandItem>
          ))}
        </CommandGroup>
      </CommandList>
    </CommandDialog>
  )
}
