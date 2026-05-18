import { useCallback } from 'react'
import { useRecents } from '../hooks/useRecents'
import { useAppStore } from '../store/app-store'
import { useOpenMarkdownFile } from '../hooks/useOpenMarkdownFile'
import { basename } from '../lib/path-utils'
import {
  SidebarGroup,
  SidebarGroupLabel,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuButton,
} from './ui/sidebar'
import { FileText } from '@phosphor-icons/react'

export function RecentsList() {
  const { data: recents = [] } = useRecents()
  const activeTab = useAppStore((s) => {
    const tab = s.tabs.find((t) => t.id === s.activeTabId)
    return tab ?? null
  })
  const openMarkdownFile = useOpenMarkdownFile()

  const handleClick = useCallback(
    async (path: string) => {
      await openMarkdownFile(path)
    },
    [openMarkdownFile],
  )

  const handleContextMenu = useCallback((path: string) => {
    void window.api.showInFolder(path)
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
                isActive={activeTab?.path === path}
                onClick={() => void handleClick(path)}
                onContextMenu={() => handleContextMenu(path)}
                title={path}
                className={activeTab?.path === path ? 'tree-file-active' : ''}
              >
                <FileText className="size-3.5 opacity-40" />
                <span className="truncate">{basename(path)}</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
          ))}
        </SidebarMenu>
      </SidebarGroupContent>
    </SidebarGroup>
  )
}
