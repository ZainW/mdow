import { useCallback } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useRecents } from '../hooks/useRecents'
import { useAppStore } from '../store/app-store'
import { basename } from '../lib/path-utils'
import {
  SidebarGroup,
  SidebarGroupLabel,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuButton,
} from './ui/sidebar'
import { FileTextIcon } from 'lucide-react'

export function RecentsList() {
  const { data: recents = [] } = useRecents()
  const setActiveFile = useAppStore((s) => s.setActiveFile)
  const activeFile = useAppStore((s) => s.activeFile)
  const queryClient = useQueryClient()

  const handleClick = useCallback(
    async (path: string) => {
      const content = await window.api.readFile(path)
      setActiveFile({ path, content })
      void queryClient.invalidateQueries({ queryKey: ['recents'] })
    },
    [setActiveFile, queryClient],
  )

  const handleContextMenu = useCallback((path: string) => {
    window.api.showInFolder(path)
  }, [])

  if (recents.length === 0) {
    return null
  }

  return (
    <SidebarGroup>
      <SidebarGroupLabel>Recents</SidebarGroupLabel>
      <SidebarGroupContent>
        <SidebarMenu>
          {recents.map((path) => (
            <SidebarMenuItem key={path}>
              <SidebarMenuButton
                isActive={activeFile?.path === path}
                onClick={() => void handleClick(path)}
                onContextMenu={() => handleContextMenu(path)}
                title={path}
                className={activeFile?.path === path ? 'tree-file-active' : ''}
              >
                <FileTextIcon className="size-3.5 opacity-40" />
                <span className="truncate">{basename(path)}</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
          ))}
        </SidebarMenu>
      </SidebarGroupContent>
    </SidebarGroup>
  )
}
