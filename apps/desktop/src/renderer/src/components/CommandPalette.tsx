import { useCallback, useMemo } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore } from '../store/app-store'
import { useRecents } from '../hooks/useRecents'
import { basename, parentDir } from '../lib/path-utils'
import {
  Command,
  CommandInput,
  CommandList,
  CommandEmpty,
  CommandGroup,
  CommandItem,
} from './ui/command'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from './ui/dialog'
import { FileText, Plus, PencilSimple } from '@phosphor-icons/react'

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

export function CommandPalette() {
  const commandPaletteOpen = useAppStore((s) => s.commandPaletteOpen)
  const setCommandPaletteOpen = useAppStore((s) => s.setCommandPaletteOpen)
  const openTab = useAppStore((s) => s.openTab)
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
      openTab({ path, content })
      void queryClient.invalidateQueries({ queryKey: ['recents'] })
    },
    [setCommandPaletteOpen, openTab, queryClient],
  )

  const handleNewDocument = useCallback(async () => {
    setCommandPaletteOpen(false)
    const folderPath = useAppStore.getState().openFolderPath
    const result = await window.api.createFile(folderPath)
    if (!result) return
    useAppStore.getState().openTab({ path: result.path, content: '' })
    const newId = useAppStore.getState().activeTabId
    if (newId) useAppStore.getState().setTabMode(newId, 'edit')
  }, [setCommandPaletteOpen])

  const handleToggleEdit = useCallback(() => {
    setCommandPaletteOpen(false)
    const id = useAppStore.getState().activeTabId
    if (id) useAppStore.getState().toggleTabMode(id)
  }, [setCommandPaletteOpen])

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
        <Command>
          <CommandInput placeholder="Search files..." />
          <CommandList>
            <CommandEmpty>No matching files</CommandEmpty>
            <CommandGroup heading="Commands">
              <CommandItem
                value="new-document"
                keywords={['new', 'create', 'document', 'file']}
                onSelect={() => void handleNewDocument()}
              >
                <Plus />
                <span>New document</span>
                <span className="ml-auto shrink-0 text-[11px] text-muted-foreground/50">Cmd+N</span>
              </CommandItem>
              <CommandItem
                value="toggle-edit"
                keywords={['edit', 'toggle', 'mode', 'writing']}
                onSelect={handleToggleEdit}
              >
                <PencilSimple />
                <span>Toggle edit mode</span>
                <span className="ml-auto shrink-0 text-[11px] text-muted-foreground/50">Cmd+E</span>
              </CommandItem>
            </CommandGroup>
            <CommandGroup heading="Files">
              {allFiles.map((file) => {
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
                      <span className="ml-auto shrink-0 text-[11px] text-muted-foreground/50">
                        {dir}
                      </span>
                    )}
                  </CommandItem>
                )
              })}
            </CommandGroup>
          </CommandList>
        </Command>
      </DialogContent>
    </Dialog>
  )
}
