import { useCallback, useEffect, useEffectEvent, useState } from 'react'
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
import { Clock, FileText } from 'lucide-react'
import { EmptyState } from './EmptyState'
import { rovingTabIndex, useRovingFocus } from '../hooks/useRovingFocus'

interface ContextMenuState {
  path: string
  x: number
  y: number
}

const revealLabel = isMac ? 'Reveal in Finder' : 'Show in Folder'

export function RecentsList() {
  const { data: recents = [] } = useRecents()
  const queryClient = useQueryClient()
  const activeTab = useAppStore((s) => {
    const tab = s.tabs.find((t) => t.id === s.activeTabId)
    return tab ?? null
  })
  const openMarkdownFile = useOpenMarkdownFile()
  const [menu, setMenu] = useState<ContextMenuState | null>(null)

  const handleClick = useCallback(
    async (path: string) => {
      await openMarkdownFile(path)
    },
    [openMarkdownFile],
  )

  const handleContextMenu = useCallback((e: React.MouseEvent, path: string) => {
    e.preventDefault()
    setMenu({ path, x: e.clientX, y: e.clientY })
  }, [])

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
    <>
      <SidebarGroup>
        <SidebarGroupLabel>Recents</SidebarGroupLabel>
        <SidebarGroupContent>
          <SidebarMenu>
            {recents.map((path) => {
              const dir = parentDir(path)
              return (
                <SidebarMenuItem key={path}>
                  <SidebarMenuButton
                    isActive={activeTab?.path === path}
                    onClick={() => void handleClick(path)}
                    onContextMenu={(e) => handleContextMenu(e, path)}
                    title={path}
                    className={cn(
                      'h-auto min-h-7 flex-col items-start gap-0 py-1.5',
                      activeTab?.path === path && 'tree-file-active',
                    )}
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
                  </SidebarMenuButton>
                </SidebarMenuItem>
              )
            })}
          </SidebarMenu>
        </SidebarGroupContent>
      </SidebarGroup>
      {menu && (
        <RecentsContextMenu
          {...menu}
          onClose={() => setMenu(null)}
          onOpen={() => void handleClick(menu.path)}
          onCopyPath={() => void navigator.clipboard.writeText(menu.path)}
          onReveal={() => void window.api.showInFolder(menu.path)}
          onRemove={() => {
            void (async () => {
              const current = await window.api.getRecents()
              const updated = current.filter((p) => p !== menu.path)
              await window.api.saveAppState({ recents: updated } as Parameters<
                typeof window.api.saveAppState
              >[0])
              void queryClient.invalidateQueries({ queryKey: ['recents'] })
            })()
          }}
        />
      )}
    </>
  )
}

interface RecentsContextMenuProps {
  x: number
  y: number
  onClose: () => void
  onOpen: () => void
  onCopyPath: () => void
  onReveal: () => void
  onRemove: () => void
}

function RecentsContextMenu({
  x,
  y,
  onClose,
  onOpen,
  onCopyPath,
  onReveal,
  onRemove,
}: RecentsContextMenuProps) {
  const menuRoving = useRovingFocus({
    orientation: 'vertical',
    autoFocusFirst: true,
  })
  const ref = menuRoving.containerRef
  const onCloseEvent = useEffectEvent(onClose)

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      const target = e.target
      if (!(target instanceof Node)) return
      if (ref.current && !ref.current.contains(target)) onCloseEvent()
    }
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault()
        onCloseEvent()
      }
    }
    document.addEventListener('mousedown', handleClickOutside)
    document.addEventListener('keydown', handleEscape)
    return () => {
      document.removeEventListener('mousedown', handleClickOutside)
      document.removeEventListener('keydown', handleEscape)
    }
  }, [ref])

  useEffect(() => {
    const el = ref.current
    if (!el) return
    const rect = el.getBoundingClientRect()
    let nx = x
    let ny = y
    if (rect.right > window.innerWidth - 8) nx = window.innerWidth - rect.width - 8
    if (rect.bottom > window.innerHeight - 8) ny = window.innerHeight - rect.height - 8
    el.style.cssText = `left:${nx}px;top:${ny}px;`
  }, [x, y, ref])

  const item = (label: string, onClick: () => void, opts: { danger?: boolean } = {}) => (
    <button
      type="button"
      role="menuitem"
      tabIndex={rovingTabIndex(false)}
      onClick={() => {
        onClick()
        onClose()
      }}
      className={cn(
        'tab-menu-item flex w-full items-center rounded-sm px-2 py-1.5 text-left text-xs outline-none focus-visible:bg-muted',
        opts.danger
          ? 'text-destructive hover:bg-destructive/10 hover:text-destructive'
          : 'text-foreground hover:bg-muted',
      )}
    >
      {label}
    </button>
  )

  return (
    // oxlint-disable-next-line jsx-a11y/interactive-supports-focus -- per WAI-ARIA, focus rests on the focused menuitem inside, not the menu itself
    <div
      ref={ref}
      role="menu"
      onKeyDown={menuRoving.onKeyDown}
      className="tab-context-menu fixed z-50 min-w-[200px] rounded-md border border-border bg-popover p-1 text-popover-foreground shadow-md"
      style={{ left: x, top: y }}
    >
      {item('Open', onOpen)}
      {item('Copy Path', onCopyPath)}
      {item(revealLabel, onReveal)}
      <div className="my-1 h-px bg-border-subtle" />
      {item('Remove from Recents', onRemove, { danger: true })}
    </div>
  )
}
