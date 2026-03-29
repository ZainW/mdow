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
import {
  Collapsible,
  CollapsibleTrigger,
  CollapsibleContent,
} from './ui/collapsible'
import { ChevronRightIcon, FileTextIcon, FolderIcon } from 'lucide-react'
import { Separator } from './ui/separator'

interface TreeNode {
  name: string
  path: string
  isDirectory: boolean
  children?: TreeNode[]
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
  return (
    <SidebarMenuItem>
      <Collapsible defaultOpen={depth === 0} className="group/collapsible">
        <CollapsibleTrigger render={<SidebarMenuButton />}>
          <ChevronRightIcon className="transition-transform group-data-[state=open]/collapsible:rotate-90" />
          <FolderIcon />
          <span className="truncate">{node.name}</span>
        </CollapsibleTrigger>
        <CollapsibleContent>
          <SidebarMenuSub>
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
  return (
    <SidebarMenuSubItem>
      <SidebarMenuSubButton
        render={<button type="button" />}
        isActive={activeFilePath === node.path}
        onClick={() => void onFileClick(node.path)}
        title={node.path}
      >
        <FileTextIcon />
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
  return (
    <TreeItemFile
      node={node}
      activeFilePath={activeFilePath}
      onFileClick={onFileClick}
    />
  )
}

export function FolderTree() {
  const folderTree = useAppStore((s) => s.folderTree)
  const openFolderPath = useAppStore((s) => s.openFolderPath)
  const activeFile = useAppStore((s) => s.activeFile)
  const setActiveFile = useAppStore((s) => s.setActiveFile)
  const queryClient = useQueryClient()

  const handleFileClick = useCallback(
    async (path: string) => {
      const content = await window.api.readFile(path)
      setActiveFile({ path, content })
      void queryClient.invalidateQueries({ queryKey: ['recents'] })
    },
    [setActiveFile, queryClient]
  )

  if (!openFolderPath || folderTree.length === 0) return null

  const folderName = openFolderPath.split(/[/\\]/).pop() || openFolderPath

  return (
    <>
      <Separator />
      <SidebarGroup>
        <SidebarGroupLabel className="flex justify-between">
          <span>Folder</span>
          <span className="text-muted-foreground font-normal normal-case opacity-60">{folderName}</span>
        </SidebarGroupLabel>
        <SidebarGroupContent>
          <SidebarMenu>
            {folderTree.map((node) => (
              <TreeItemNode
                key={node.path}
                node={node}
                activeFilePath={activeFile?.path ?? null}
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
