import { useCallback, useEffect, useMemo, useState } from 'react'
import type { TreeNode } from '../../../shared/types'
import { useAppStore } from '../store/app-store'
import { useOpenMarkdownFile } from '../hooks/useOpenMarkdownFile'
import { useOpenFileDialog } from '../hooks/useOpenFileDialog'
import { useOpenFolderDialog } from '../hooks/useOpenFolderDialog'
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
  CommandShortcut,
} from './ui/command'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from './ui/dialog'
import { Kbd } from './ui/kbd'
import {
  ArrowLeftRight,
  Columns2,
  FilePlus,
  FileSearch,
  FileText,
  FolderOpen,
  Keyboard,
  PanelLeft,
  Settings,
} from 'lucide-react'

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

interface PaletteAction {
  id: string
  label: string
  hint: string
  keywords: string[]
  Icon: typeof Settings
  run: () => void | Promise<void>
}

function filterActions(query: string, actions: PaletteAction[]): PaletteAction[] {
  const trimmed = query.trim()
  if (!trimmed) return actions

  const searchItems = actions.map((action) => ({
    path: [action.label, action.hint, ...action.keywords].join(' '),
    name: action.label,
  }))
  const resultLabels = new Set(fuzzySearch(trimmed, searchItems, actions.length).map((r) => r.name))
  return actions.filter((action) => resultLabels.has(action.label))
}

export function CommandPalette() {
  const commandPaletteOpen = useAppStore((s) => s.commandPaletteOpen)
  const setCommandPaletteOpen = useAppStore((s) => s.setCommandPaletteOpen)
  const folderTree = useAppStore((s) => s.folderTree)
  const hasTabs = useAppStore((s) => s.tabs.length > 0)
  const { data: recents = [] } = useRecents()
  const openMarkdownFile = useOpenMarkdownFile()
  const openFileDialog = useOpenFileDialog()
  const openFolderDialog = useOpenFolderDialog()
  const setSettingsOpen = useAppStore((s) => s.setSettingsOpen)
  const setShortcutsDialogOpen = useAppStore((s) => s.setShortcutsDialogOpen)
  const setSearchOpen = useAppStore((s) => s.setSearchOpen)
  const toggleSidebar = useAppStore((s) => s.toggleSidebar)
  const toggleWideMode = useAppStore((s) => s.toggleWideMode)
  const toggleSplitView = useAppStore((s) => s.toggleSplitView)
  const [query, setQuery] = useState('')
  const allFiles = useMemo(() => {
    if (!commandPaletteOpen) return null
    return buildAllFiles(folderTree, recents)
  }, [commandPaletteOpen, folderTree, recents])
  const actions = useMemo(
    () =>
      [
        {
          id: 'open-file',
          label: 'Open File',
          hint: 'Choose a markdown or HTML document',
          keywords: ['document', 'markdown', 'html'],
          Icon: FilePlus,
          run: openFileDialog,
        },
        {
          id: 'open-folder',
          label: 'Open Folder',
          hint: 'Browse a folder of documents',
          keywords: ['workspace', 'directory', 'sidebar'],
          Icon: FolderOpen,
          run: openFolderDialog,
        },
        {
          id: 'toggle-sidebar',
          label: 'Toggle Sidebar',
          hint: 'Show or hide navigation',
          keywords: ['recents', 'folder', 'outline'],
          Icon: PanelLeft,
          run: toggleSidebar,
        },
        {
          id: 'find',
          label: 'Find in Document',
          hint: 'Search the active document',
          keywords: ['search', 'matches'],
          Icon: FileSearch,
          run: () => setSearchOpen(true),
        },
        {
          id: 'toggle-wide-mode',
          label: 'Toggle Wide Mode',
          hint: 'Switch reading width',
          keywords: ['full width', 'reading', 'layout'],
          Icon: ArrowLeftRight,
          run: toggleWideMode,
        },
        {
          id: 'toggle-split-view',
          label: 'Toggle Split View',
          hint: hasTabs ? 'Read side by side' : 'Open a document first',
          keywords: ['side by side', 'pane', 'columns'],
          Icon: Columns2,
          run: toggleSplitView,
        },
        {
          id: 'settings',
          label: 'Settings',
          hint: 'Theme, fonts, updates',
          keywords: ['preferences', 'appearance'],
          Icon: Settings,
          run: () => setSettingsOpen(true),
        },
        {
          id: 'shortcuts',
          label: 'Keyboard Shortcuts',
          hint: 'View available shortcuts',
          keywords: ['help', 'keys'],
          Icon: Keyboard,
          run: () => setShortcutsDialogOpen(true),
        },
      ] satisfies PaletteAction[],
    [
      hasTabs,
      openFileDialog,
      openFolderDialog,
      setSearchOpen,
      setSettingsOpen,
      setShortcutsDialogOpen,
      toggleSidebar,
      toggleSplitView,
      toggleWideMode,
    ],
  )

  useEffect(() => {
    if (!commandPaletteOpen) setQuery('')
  }, [commandPaletteOpen])

  const results = useMemo(() => {
    if (!allFiles) return []
    return fuzzySearch(query, allFiles, 50)
  }, [query, allFiles])

  const actionResults = useMemo(() => filterActions(query, actions), [query, actions])

  const selectFile = useCallback(
    async (path: string) => {
      setCommandPaletteOpen(false)
      await openMarkdownFile(path)
    },
    [setCommandPaletteOpen, openMarkdownFile],
  )

  const selectAction = useCallback(
    async (action: PaletteAction) => {
      setCommandPaletteOpen(false)
      await action.run()
    },
    [setCommandPaletteOpen],
  )

  return (
    <Dialog open={commandPaletteOpen} onOpenChange={setCommandPaletteOpen}>
      <DialogContent
        className="top-[20%] translate-y-0 overflow-hidden rounded-xl p-0 sm:max-w-lg"
        showCloseButton={false}
      >
        <DialogHeader className="sr-only">
          <DialogTitle>Command Palette</DialogTitle>
          <DialogDescription>Search for files and commands</DialogDescription>
        </DialogHeader>
        <Command shouldFilter={false}>
          <CommandInput
            value={query}
            onValueChange={setQuery}
            placeholder="Search files and commands..."
          />
          <CommandList>
            <CommandEmpty>No matching files or commands</CommandEmpty>
            {actionResults.length > 0 && (
              <CommandGroup heading="Actions">
                {actionResults.map((action) => (
                  <CommandItem
                    key={action.id}
                    value={`action:${action.id}`}
                    keywords={[action.label, action.hint, ...action.keywords]}
                    onSelect={() => void selectAction(action)}
                  >
                    <action.Icon />
                    <span className="min-w-0 truncate">{action.label}</span>
                    <CommandShortcut className="tracking-normal">{action.hint}</CommandShortcut>
                  </CommandItem>
                ))}
              </CommandGroup>
            )}
            {results.length > 0 && (
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
            )}
          </CommandList>
          <div className="flex items-center justify-end gap-3 border-t border-border-subtle bg-muted/40 px-3 py-1.5 text-[10px] text-muted-foreground/80">
            <span className="flex items-center gap-1">
              <Kbd>↵</Kbd> run/open
            </span>
            <span className="flex items-center gap-1">
              <Kbd>esc</Kbd> dismiss
            </span>
          </div>
        </Command>
      </DialogContent>
    </Dialog>
  )
}
