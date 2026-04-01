import { useCallback, useMemo, useRef, useState } from 'react'
import { useVirtualizer } from '@tanstack/react-virtual'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore } from '../store/app-store'
import { SidebarGroup, SidebarGroupLabel, SidebarGroupContent } from './ui/sidebar'
import { CaretRight, FileText, Folder, FolderOpen } from '@phosphor-icons/react'
import { Separator } from './ui/separator'

interface TreeNode {
  name: string
  path: string
  isDirectory: boolean
  children?: TreeNode[]
}

interface FlatNode {
  node: TreeNode
  depth: number
}

function collectAncestorPaths(nodes: TreeNode[], targetPath: string, result: Set<string>): boolean {
  for (const node of nodes) {
    if (node.path === targetPath) return true
    if (node.isDirectory && node.children) {
      if (collectAncestorPaths(node.children, targetPath, result)) {
        result.add(node.path)
        return true
      }
    }
  }
  return false
}

function flattenVisible(
  nodes: TreeNode[],
  expanded: Set<string>,
  depth: number,
  result: FlatNode[],
): void {
  for (const node of nodes) {
    result.push({ node, depth })
    if (node.isDirectory && expanded.has(node.path) && node.children) {
      flattenVisible(node.children, expanded, depth + 1, result)
    }
  }
}

const ROW_HEIGHT = 28

export function FolderTree() {
  const folderTree = useAppStore((s) => s.folderTree)
  const openFolderPath = useAppStore((s) => s.openFolderPath)
  const activeTab = useAppStore((s) => {
    const tab = s.tabs.find((t) => t.id === s.activeTabId)
    return tab ?? null
  })
  const openTab = useAppStore((s) => s.openTab)
  const queryClient = useQueryClient()

  // Auto-expand directories containing the active file
  const autoExpanded = useMemo(() => {
    const paths = new Set<string>()
    if (activeTab?.path) {
      collectAncestorPaths(folderTree, activeTab.path, paths)
    }
    return paths
  }, [folderTree, activeTab?.path])

  const [manualExpanded, setManualExpanded] = useState<Set<string>>(new Set())
  const [manualCollapsed, setManualCollapsed] = useState<Set<string>>(new Set())

  // Effective expanded = (auto-expanded OR manually expanded) AND NOT manually collapsed
  const expanded = useMemo(() => {
    const set = new Set<string>()
    for (const p of autoExpanded) {
      if (!manualCollapsed.has(p)) set.add(p)
    }
    for (const p of manualExpanded) {
      if (!manualCollapsed.has(p)) set.add(p)
    }
    return set
  }, [autoExpanded, manualExpanded, manualCollapsed])

  const flatNodes = useMemo(() => {
    const result: FlatNode[] = []
    flattenVisible(folderTree, expanded, 0, result)
    return result
  }, [folderTree, expanded])

  const scrollRef = useRef<HTMLDivElement>(null)
  const virtualizer = useVirtualizer({
    count: flatNodes.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 20,
  })

  const toggleExpand = useCallback(
    (path: string) => {
      if (expanded.has(path)) {
        // Collapse: add to manualCollapsed, remove from manualExpanded
        setManualCollapsed((prev) => new Set(prev).add(path))
        setManualExpanded((prev) => {
          const next = new Set(prev)
          next.delete(path)
          return next
        })
      } else {
        // Expand: add to manualExpanded, remove from manualCollapsed
        setManualExpanded((prev) => new Set(prev).add(path))
        setManualCollapsed((prev) => {
          const next = new Set(prev)
          next.delete(path)
          return next
        })
      }
    },
    [expanded],
  )

  const handleFileClick = useCallback(
    (path: string) => {
      void window.api.readFile(path).then((content) => {
        openTab({ path, content })
        void queryClient.invalidateQueries({ queryKey: ['recents'] })
      })
    },
    [openTab, queryClient],
  )

  if (!openFolderPath || folderTree.length === 0) return null

  const folderName = openFolderPath.split(/[/\\]/).pop() || openFolderPath

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
          <div ref={scrollRef} className="h-full overflow-y-auto no-scrollbar">
            <div className="relative w-full" style={{ height: virtualizer.getTotalSize() }}>
              {virtualizer.getVirtualItems().map((virtualRow) => {
                const { node, depth } = flatNodes[virtualRow.index]
                const isActive = activeTab?.path === node.path
                const isExpanded = expanded.has(node.path)

                if (node.isDirectory) {
                  return (
                    <button
                      type="button"
                      key={node.path}
                      className="absolute left-0 flex w-full items-center gap-1.5 rounded-md px-2 text-sm text-sidebar-foreground hover:bg-sidebar-accent/70"
                      style={{
                        height: ROW_HEIGHT,
                        top: virtualRow.start,
                        paddingLeft: 8 + depth * 12,
                      }}
                      onClick={() => toggleExpand(node.path)}
                    >
                      <CaretRight
                        className="size-3.5 shrink-0 opacity-50 transition-transform"
                        style={{
                          transform: isExpanded ? 'rotate(90deg)' : undefined,
                        }}
                      />
                      {isExpanded ? (
                        <FolderOpen className="size-4 shrink-0 text-sidebar-primary/70" />
                      ) : (
                        <Folder className="size-4 shrink-0 text-sidebar-primary/70" />
                      )}
                      <span className="truncate">{node.name}</span>
                    </button>
                  )
                }

                return (
                  <button
                    type="button"
                    key={node.path}
                    className={`absolute left-0 flex w-full items-center gap-2 rounded-md px-2 text-sm text-sidebar-foreground hover:bg-sidebar-accent/70 ${isActive ? 'bg-sidebar-accent text-sidebar-accent-foreground' : ''}`}
                    style={{
                      height: ROW_HEIGHT,
                      top: virtualRow.start,
                      paddingLeft: 8 + depth * 12,
                    }}
                    title={node.path}
                    onClick={() => handleFileClick(node.path)}
                  >
                    <FileText className="size-3.5 shrink-0 opacity-40" />
                    <span className="truncate">{node.name}</span>
                  </button>
                )
              })}
            </div>
          </div>
        </SidebarGroupContent>
      </SidebarGroup>
    </>
  )
}
