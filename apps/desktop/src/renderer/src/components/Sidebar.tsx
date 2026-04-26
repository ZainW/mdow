import { useCallback, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore } from '../store/app-store'
import { RecentsList } from './RecentsList'
import { FolderTree } from './FolderTree'
import { Button } from './ui/button'
import {
  Sidebar as ShadcnSidebar,
  SidebarContent,
  SidebarHeader,
  SidebarGroup,
  SidebarGroupContent,
} from './ui/sidebar'
import {
  Clock,
  File,
  Folder,
  FolderOpen,
  GearSix,
  ListBullets,
  MagnifyingGlass,
} from '@phosphor-icons/react'
import type { DocHeading } from '../lib/markdown'

type RailMode = 'recents' | 'folder' | 'outline'

const MODES: RailMode[] = ['recents', 'folder', 'outline']
const DRAWER_WIDTH = 244

// Rail layout — keep in sync with the JSX (icons are h-7 with gap-0.5, py-1.5 padding)
const ICON_HEIGHT = 28
const ICON_GAP = 2
const RAIL_PAD_TOP = 6
const INDICATOR_HEIGHT = 16

function indicatorY(modeIndex: number): number {
  const iconTop = RAIL_PAD_TOP + modeIndex * (ICON_HEIGHT + ICON_GAP)
  return iconTop + (ICON_HEIGHT - INDICATOR_HEIGHT) / 2
}

export function Sidebar() {
  const sidebarOpen = useAppStore((s) => s.sidebarOpen)
  const setCommandPaletteOpen = useAppStore((s) => s.setCommandPaletteOpen)
  const setSettingsOpen = useAppStore((s) => s.setSettingsOpen)
  const openTab = useAppStore((s) => s.openTab)
  const setOpenFolder = useAppStore((s) => s.setOpenFolder)
  const docHeadings = useAppStore((s) => s.docHeadings)
  const activeHeadingId = useAppStore((s) => s.activeHeadingId)
  const hasOpenTab = useAppStore((s) => s.tabs.length > 0)
  const queryClient = useQueryClient()
  const [mode, setMode] = useState<RailMode>('recents')

  const handleOpenFile = useCallback(async () => {
    const result = await window.api.openFileDialog()
    if (result) {
      openTab(result)
      void queryClient.invalidateQueries({ queryKey: ['recents'] })
    }
  }, [openTab, queryClient])

  const handleOpenFolder = useCallback(async () => {
    const result = await window.api.openFolderDialog()
    if (result) {
      setOpenFolder(result.path, result.tree)
    }
  }, [setOpenFolder])

  const drawerTitle = mode === 'recents' ? 'Recents' : mode === 'folder' ? 'Folder' : 'Outline'
  const modeIndex = MODES.indexOf(mode)

  return (
    <div className="flex h-full shrink-0 border-r border-border-subtle">
      <div className="relative flex w-9 shrink-0 flex-col items-center gap-0.5 border-r border-border-subtle bg-sidebar/40 py-1.5">
        <span
          aria-hidden
          className="rail-indicator pointer-events-none absolute left-0 w-0.5 rounded-r bg-primary"
          style={{
            height: INDICATOR_HEIGHT,
            top: 0,
            transform: `translateY(${indicatorY(modeIndex)}px)`,
          }}
        />
        <RailIcon active={mode === 'recents'} onClick={() => setMode('recents')} label="Recents">
          <Clock />
        </RailIcon>
        <RailIcon active={mode === 'folder'} onClick={() => setMode('folder')} label="Folder">
          <Folder />
        </RailIcon>
        <RailIcon active={mode === 'outline'} onClick={() => setMode('outline')} label="Outline">
          <ListBullets />
        </RailIcon>
        <RailIcon active={false} onClick={() => setCommandPaletteOpen(true)} label="Quick Open">
          <MagnifyingGlass />
        </RailIcon>
        <div className="flex-1" />
        <RailIcon active={false} onClick={() => void handleOpenFile()} label="Open File">
          <File />
        </RailIcon>
        <RailIcon active={false} onClick={() => void handleOpenFolder()} label="Open Folder">
          <FolderOpen />
        </RailIcon>
        <RailIcon active={false} onClick={() => setSettingsOpen(true)} label="Settings">
          <GearSix />
        </RailIcon>
      </div>

      <div
        className="shrink-0 overflow-hidden"
        style={{
          width: sidebarOpen ? DRAWER_WIDTH : 0,
          transition: 'width 200ms var(--ease-out-ui)',
        }}
      >
        <ShadcnSidebar
          collapsible="none"
          className="h-full border-none"
          style={{ width: DRAWER_WIDTH }}
        >
          <SidebarHeader className="px-3 py-2">
            <div className="flex h-7 items-center">
              <span className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
                {drawerTitle}
              </span>
            </div>
          </SidebarHeader>
          <SidebarContent key={mode} className="drawer-mode">
            {mode === 'recents' && <RecentsList />}
            {mode === 'folder' && <FolderTree />}
            {mode === 'outline' && (
              <OutlineList
                headings={docHeadings}
                activeId={activeHeadingId}
                hasActiveDoc={hasOpenTab}
              />
            )}
          </SidebarContent>
        </ShadcnSidebar>
      </div>
    </div>
  )
}

function RailIcon({
  active,
  onClick,
  label,
  children,
}: {
  active: boolean
  onClick: () => void
  label: string
  children: React.ReactNode
}) {
  return (
    <Button
      variant="ghost"
      size="icon"
      aria-label={label}
      title={label}
      aria-pressed={active}
      className={`rail-icon-btn h-7 w-7 hover:bg-transparent ${active ? 'text-foreground' : 'text-muted-foreground hover:text-foreground'}`}
      onClick={onClick}
    >
      <span className="[&>svg]:size-4">{children}</span>
    </Button>
  )
}

function OutlineList({
  headings,
  activeId,
  hasActiveDoc,
}: {
  headings: DocHeading[]
  activeId: string | null
  hasActiveDoc: boolean
}) {
  const handleClick = useCallback((e: React.MouseEvent<HTMLAnchorElement>, id: string) => {
    e.preventDefault()
    const el = document.getElementById(id)
    if (!el) return
    const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches
    el.scrollIntoView({ behavior: reduceMotion ? 'auto' : 'smooth', block: 'start' })
  }, [])

  if (headings.length === 0) {
    return (
      <SidebarGroup>
        <SidebarGroupContent>
          <p className="px-3 py-2 text-xs text-muted-foreground/70">
            {hasActiveDoc ? 'No headings in this document.' : 'Open a document to see its outline.'}
          </p>
        </SidebarGroupContent>
      </SidebarGroup>
    )
  }
  return (
    <SidebarGroup>
      <SidebarGroupContent>
        <ul role="list" className="flex flex-col gap-px px-1.5 py-1">
          {headings.map((h) => {
            const isActive = h.id === activeId
            return (
              <li key={h.id}>
                <a
                  href={`#${h.id}`}
                  data-active={isActive}
                  onClick={(e) => handleClick(e, h.id)}
                  className="outline-link block truncate rounded px-1.5 py-1 text-xs text-sidebar-foreground/75 hover:bg-sidebar-accent/60 hover:text-foreground"
                  style={{ paddingLeft: 6 + (h.level - 1) * 10 }}
                  title={h.text}
                >
                  {h.text}
                </a>
              </li>
            )
          })}
        </ul>
      </SidebarGroupContent>
    </SidebarGroup>
  )
}
