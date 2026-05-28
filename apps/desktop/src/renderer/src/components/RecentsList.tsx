import { useCallback } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useRecents } from '../hooks/useRecents'
import { useAppStore } from '../store/app-store'
import { useOpenMarkdownFile } from '../hooks/useOpenMarkdownFile'
import { basename, parentDir } from '../lib/path-utils'
import { cn, isMac } from '../lib/utils'
import {
  SidebarGroup,
  SidebarGroupLabel,
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
        const updated = current.filter((p) => p !== path)
        await window.api.saveAppState({ recents: updated } as Parameters<
          typeof window.api.saveAppState
        >[0])
        void queryClient.invalidateQueries({ queryKey: ['recents'] })
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
    <SidebarGroup>
      <SidebarGroupLabel>Recents</SidebarGroupLabel>
      <SidebarGroupContent>
        <SidebarMenu>
          {recents.map((path) => {
            const dir = parentDir(path)
            return (
              <SidebarMenuItem key={path}>
                <ContextMenu>
                  <ContextMenuTrigger
                    render={
                      <SidebarMenuButton
                        isActive={activeTab?.path === path}
                        onClick={() => void handleClick(path)}
                        title={path}
                        className={cn(
                          'h-auto min-h-7 flex-col items-start gap-0 py-1.5',
                          activeTab?.path === path && 'tree-file-active',
                        )}
                      />
                    }
                  >
                    <span className="flex w-full min-w-0 items-center gap-2">
                      <FileText className="size-3.5 shrink-0 opacity-40" />
                      <span className="truncate">{basename(path)}</span>
                    </span>
                    {dir && (
                      <span className="w-full truncate pl-5 text-[10px] text-muted-foreground/60">
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
