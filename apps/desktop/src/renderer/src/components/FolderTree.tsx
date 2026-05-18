import { useEffect, useMemo, useReducer, useRef } from 'react'
import { FileTree as FileTreeModel } from '@pierre/trees'
import type { FileTreeOptions } from '@pierre/trees'
import { FileTree as FileTreeView } from '@pierre/trees/react'
import { useAppStore } from '../store/app-store'
import { useOpenMarkdownFile } from '../hooks/useOpenMarkdownFile'
import { basename } from '../lib/path-utils'
import { SidebarGroup, SidebarGroupLabel, SidebarGroupContent } from './ui/sidebar'
import { Separator } from './ui/separator'

type DirectoryHandle = ReturnType<FileTreeModel['getItem']> & { expand(): void }

interface TreeNode {
  name: string
  path: string
  isDirectory: boolean
  children?: TreeNode[]
}

function normalizeRoot(root: string): string {
  return root.replace(/\\/g, '/').replace(/\/+$/, '')
}

function toRelative(absPath: string, normalizedRoot: string): string {
  const a = absPath.replace(/\\/g, '/')
  if (a === normalizedRoot) return ''
  if (a.startsWith(normalizedRoot + '/')) return a.slice(normalizedRoot.length + 1)
  return a
}

function collectPaths(nodes: TreeNode[], normalizedRoot: string, out: string[]): void {
  for (const node of nodes) {
    const rel = toRelative(node.path, normalizedRoot)
    if (!rel) continue
    if (node.isDirectory) {
      if (node.children && node.children.length > 0) {
        collectPaths(node.children, normalizedRoot, out)
      } else {
        out.push(rel + '/')
      }
    } else {
      out.push(rel)
    }
  }
}

function ancestorRelPaths(rel: string): string[] {
  const parts = rel.split('/')
  const out: string[] = []
  let acc = ''
  for (let i = 0; i < parts.length - 1; i++) {
    acc = acc ? `${acc}/${parts[i]}` : parts[i]
    out.push(acc)
  }
  return out
}

function relToAbsolute(rel: string, openFolderPath: string): string {
  const sep = openFolderPath.includes('\\') ? '\\' : '/'
  const nativeRel = sep === '\\' ? rel.replace(/\//g, '\\') : rel
  return openFolderPath.replace(/[/\\]$/, '') + sep + nativeRel
}

// Strict-mode-safe model lifecycle: in StrictMode, React runs mount → unmount → mount
// on first commit. The library's own useFileTree hook destroys the model on the
// strict-mode unmount and never recreates it, leaving us with a dead controller whose
// selection subscription has been torn down. We defer cleanup to a microtask so the
// immediate strict-mode remount can cancel it.
function useStableFileTree(buildOptions: () => FileTreeOptions): FileTreeModel {
  const modelRef = useRef<FileTreeModel | null>(null)
  const pendingCleanupRef = useRef<{ cancelled: boolean } | null>(null)
  const [, forceRerender] = useReducer((x: number) => x + 1, 0)

  if (modelRef.current === null) {
    modelRef.current = new FileTreeModel(buildOptions())
  }

  useEffect(() => {
    // Cancel any cleanup left over from a strict-mode unmount that's now being
    // followed by this remount.
    const pending = pendingCleanupRef.current
    if (pending !== null) {
      pending.cancelled = true
      pendingCleanupRef.current = null
    }
    if (modelRef.current === null) {
      modelRef.current = new FileTreeModel(buildOptions())
      forceRerender()
    }
    return () => {
      const token = { cancelled: false }
      pendingCleanupRef.current = token
      queueMicrotask(() => {
        if (token.cancelled) return
        modelRef.current?.cleanUp()
        modelRef.current = null
        pendingCleanupRef.current = null
      })
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  return modelRef.current
}

export function FolderTree() {
  const folderTree = useAppStore((s) => s.folderTree)
  const folderTreeTruncated = useAppStore((s) => s.folderTreeTruncated)
  const openFolderPath = useAppStore((s) => s.openFolderPath)
  const activeTab = useAppStore((s) => {
    const tab = s.tabs.find((t) => t.id === s.activeTabId)
    return tab ?? null
  })
  const openMarkdownFile = useOpenMarkdownFile()

  const normalizedRoot = useMemo(
    () => (openFolderPath ? normalizeRoot(openFolderPath) : ''),
    [openFolderPath],
  )

  const paths = useMemo(() => {
    if (!normalizedRoot) return [] as string[]
    const out: string[] = []
    collectPaths(folderTree, normalizedRoot, out)
    return out
  }, [folderTree, normalizedRoot])

  const lastSelectionRef = useRef<string | null>(null)
  const selectionHandlerRef = useRef<(selected: readonly string[]) => void>(() => {})

  const model = useStableFileTree(() => ({
    paths,
    initialExpansion: 'closed',
    onSelectionChange: (selected) => selectionHandlerRef.current(selected),
  }))

  selectionHandlerRef.current = (selected) => {
    if (selected.length === 0) return
    const sel = selected[selected.length - 1]
    if (!sel || sel === lastSelectionRef.current) return
    if (sel.endsWith('/')) return
    const item = model.getItem(sel)
    if (item && item.isDirectory()) return
    lastSelectionRef.current = sel
    if (!openFolderPath) return
    const abs = relToAbsolute(sel, openFolderPath)
    void openMarkdownFile(abs)
  }

  // Keep the model's paths in sync as the user opens different folders or files
  // are added/removed by the watcher.
  const previousPathsRef = useRef(paths)
  useEffect(() => {
    if (previousPathsRef.current === paths) return
    previousPathsRef.current = paths
    model.resetPaths(paths)
  }, [paths, model])

  useEffect(() => {
    if (!activeTab?.path || !normalizedRoot) return
    const rel = toRelative(activeTab.path, normalizedRoot)
    if (!rel) return
    for (const ancestor of ancestorRelPaths(rel)) {
      const handle =
        getDirectoryHandle(model, ancestor) ?? getDirectoryHandle(model, ancestor + '/')
      handle?.expand()
    }
    const fileHandle = model.getItem(rel)
    if (fileHandle && !fileHandle.isSelected()) {
      lastSelectionRef.current = rel
      fileHandle.select()
    }
  }, [activeTab?.path, normalizedRoot, model, paths])

  if (!openFolderPath || folderTree.length === 0) return null

  const folderName = basename(openFolderPath)

  return (
    <>
      <Separator />
      <SidebarGroup className="flex flex-1 flex-col overflow-hidden">
        <SidebarGroupLabel className="flex shrink-0 justify-between">
          <span>Folder</span>
          <span className="font-normal normal-case text-muted-foreground opacity-60">
            {folderName}
          </span>
        </SidebarGroupLabel>
        <SidebarGroupContent className="flex-1 overflow-hidden">
          {folderTreeTruncated && (
            <div
              className="mx-2 mb-1 shrink-0 rounded-md border border-border-subtle bg-muted/40 px-2 py-1.5 text-[11px] leading-snug text-muted-foreground"
              title="Some files were hidden to keep the app responsive. Open a smaller subfolder to see them."
            >
              Large folder — some files hidden
            </div>
          )}
          <div className="folder-tree-host h-full">
            <FileTreeView model={model} style={{ height: '100%' }} />
          </div>
        </SidebarGroupContent>
      </SidebarGroup>
    </>
  )
}

function isDirectoryHandle(item: ReturnType<FileTreeModel['getItem']>): item is DirectoryHandle {
  return item != null && item.isDirectory()
}

function getDirectoryHandle(model: FileTreeModel, path: string): DirectoryHandle | null {
  const item = model.getItem(path)
  return isDirectoryHandle(item) ? item : null
}
