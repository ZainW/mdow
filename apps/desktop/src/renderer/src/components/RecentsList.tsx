import { useCallback } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useRecents } from '../hooks/useRecents'
import { useAppStore } from '../store/app-store'
import { useOpenMarkdownFile } from '../hooks/useOpenMarkdownFile'
import { basename, parentDir } from '../lib/path-utils'
import { invalidateRecents } from '../lib/query-keys'
import { cn, isMac } from '../lib/utils'
import {
  SidebarGroup,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuButton,
} from './ui/sidebar'
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from './ui/context-menu'
import { Clock, FileText } from 'lucide-react'
import { EmptyState } from './EmptyState'

const revealLabel = isMac ? 'Reveal in Finder' : 'Show in Folder'

export function RecentsList() {
  const { data: recents = [] } = useRecents()
  const queryClient = useQueryClient()
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

  const handleRemove = useCallback(
    (path: string) => {
      void (async () => {
        const current = await window.api.getRecents()
        await window.api.saveAppState({ recents: current.filter((p) => p !== path) })
        invalidateRecents(queryClient)
      })()
    },
    [queryClient],
  )

  if (recents.length === 0) {
    return (
      <EmptyState
        size="sm"
        icon={Clock}
        title="No recents yet"
        hint="Files you open will appear here."
      />
    )
  }

  return (
    <SidebarGroup className="pt-0">
      <SidebarGroupContent>
        <SidebarMenu>
          {recents.map((path) => {
            // The immediate parent folder is what tells two README.md files apart.
            const dir = parentDir(path, 1)
            const isActive = activeTab?.path === path
            return (
              <SidebarMenuItem key={path}>
                <ContextMenu>
                  <ContextMenuTrigger
                    render={
                      <SidebarMenuButton
                        isActive={isActive}
                        onClick={() => void handleClick(path)}
                        title={path}
                        className={cn('h-8 gap-2', isActive && 'tree-file-active')}
                      />
                    }
                  >
                    <FileText className="size-3.5 shrink-0 text-muted-foreground" aria-hidden />
                    <span className="min-w-0 flex-1 truncate text-foreground">
                      {basename(path)}
                    </span>
                    {dir && (
                      <span className="max-w-[45%] shrink-0 truncate text-[11px] text-muted-foreground">
                        {dir}
                      </span>
                    )}
                  </ContextMenuTrigger>
                  <ContextMenuContent className="min-w-[200px]">
                    <ContextMenuItem onClick={() => void handleClick(path)}>Open</ContextMenuItem>
                    <ContextMenuItem onClick={() => void navigator.clipboard.writeText(path)}>
                      Copy Path
                    </ContextMenuItem>
                    <ContextMenuItem onClick={() => void window.api.showInFolder(path)}>
                      {revealLabel}
                    </ContextMenuItem>
                    <ContextMenuSeparator />
                    <ContextMenuItem variant="destructive" onClick={() => handleRemove(path)}>
                      Remove from Recents
                    </ContextMenuItem>
                  </ContextMenuContent>
                </ContextMenu>
              </SidebarMenuItem>
            )
          })}
        </SidebarMenu>
      </SidebarGroupContent>
    </SidebarGroup>
  )
}
