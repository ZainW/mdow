import { useCallback } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore } from '../store/app-store'
import {
  SidebarGroup,
  SidebarGroupLabel,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuButton,
  SidebarMenuSub,
  SidebarMenuSubItem,
  SidebarMenuSubButton,
} from './ui/sidebar'
import { Collapsible, CollapsibleTrigger, CollapsibleContent } from './ui/collapsible'
import { CaretRight, FileText, Folder, FolderOpen } from '@phosphor-icons/react'
import { Separator } from './ui/separator'

interface TreeNode {
  name: string
  path: string
  isDirectory: boolean
  children?: TreeNode[]
}

function containsPath(node: TreeNode, filePath: string): boolean {
  if (!node.isDirectory) return node.path === filePath
  return node.children?.some((child) => containsPath(child, filePath)) ?? false
}

function TreeItemDirectory({
  node,
  activeFilePath,
  onFileClick,
  depth,
}: {
  node: TreeNode
  activeFilePath: string | null
  onFileClick: (path: string) => void
  depth: number
}) {
  const shouldOpen = activeFilePath ? containsPath(node, activeFilePath) : false

  return (
    <SidebarMenuItem>
      <Collapsible defaultOpen={shouldOpen} className="group/collapsible">
        <CollapsibleTrigger render={<SidebarMenuButton className="tree-folder-btn" />}>
          <CaretRight className="tree-chevron size-3.5 opacity-50" />
          <Folder className="tree-folder-icon-closed size-4 text-sidebar-primary/70" />
          <FolderOpen className="tree-folder-icon-open size-4 text-sidebar-primary/70" />
          <span className="truncate">{node.name}</span>
        </CollapsibleTrigger>
        <CollapsibleContent className="tree-collapsible-content">
          <SidebarMenuSub className="border-sidebar-border/50">
            {node.children?.map((child) => (
              <TreeItemNode
                key={child.path}
                node={child}
                activeFilePath={activeFilePath}
                onFileClick={onFileClick}
                depth={depth + 1}
              />
            ))}
          </SidebarMenuSub>
        </CollapsibleContent>
      </Collapsible>
    </SidebarMenuItem>
  )
}

function TreeItemFile({
  node,
  activeFilePath,
  onFileClick,
}: {
  node: TreeNode
  activeFilePath: string | null
  onFileClick: (path: string) => void
}) {
  const isActive = activeFilePath === node.path
  return (
    <SidebarMenuSubItem>
      <SidebarMenuSubButton
        render={<button type="button" />}
        isActive={isActive}
        onClick={() => onFileClick(node.path)}
        title={node.path}
        className={isActive ? 'tree-file-active' : ''}
      >
        <FileText className="size-3.5 opacity-40" />
        <span className="truncate">{node.name}</span>
      </SidebarMenuSubButton>
    </SidebarMenuSubItem>
  )
}

function TreeItemNode({
  node,
  activeFilePath,
  onFileClick,
  depth,
}: {
  node: TreeNode
  activeFilePath: string | null
  onFileClick: (path: string) => void
  depth: number
}) {
  if (node.isDirectory) {
    return (
      <TreeItemDirectory
        node={node}
        activeFilePath={activeFilePath}
        onFileClick={onFileClick}
        depth={depth}
      />
    )
  }
  return <TreeItemFile node={node} activeFilePath={activeFilePath} onFileClick={onFileClick} />
}

export function FolderTree() {
  const folderTree = useAppStore((s) => s.folderTree)
  const openFolderPath = useAppStore((s) => s.openFolderPath)
  const activeTab = useAppStore((s) => {
    const tab = s.tabs.find((t) => t.id === s.activeTabId)
    return tab ?? null
  })
  const openTab = useAppStore((s) => s.openTab)
  const queryClient = useQueryClient()

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
      <SidebarGroup>
        <SidebarGroupLabel className="flex justify-between">
          <span>Folder</span>
          <span className="text-muted-foreground font-normal normal-case opacity-60">
            {folderName}
          </span>
        </SidebarGroupLabel>
        <SidebarGroupContent>
          <SidebarMenu>
            {folderTree.map((node) => (
              <TreeItemNode
                key={node.path}
                node={node}
                activeFilePath={activeTab?.path ?? null}
                onFileClick={handleFileClick}
                depth={0}
              />
            ))}
          </SidebarMenu>
        </SidebarGroupContent>
      </SidebarGroup>
    </>
  )
}
