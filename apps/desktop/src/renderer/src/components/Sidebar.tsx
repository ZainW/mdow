import { useCallback } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useAppStore, type SidebarMode } from '../store/app-store'
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
import { Clock, File, Folder, FolderOpen, List, Search, Settings } from 'lucide-react'
import type { DocHeading } from '../lib/markdown'
import { EmptyState } from './EmptyState'
import { rovingTabIndex, useRovingFocus } from '../hooks/useRovingFocus'
import { isMac } from '../lib/utils'

const MODES: SidebarMode[] = ['recents', 'folder', 'outline']
const revealLabel = isMac ? 'Reveal in Finder' : 'Show in Folder'

function indicatorY(modeIndex: number): string {
  const steps = Array.from({ length: modeIndex }, () => 'var(--rail-step)').join(' + ')
  return `calc(var(--rail-pad-y) + var(--rail-indicator-offset)${steps ? ` + ${steps}` : ''})`
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
  const openFolderPath = useAppStore((s) => s.openFolderPath)
  const mode = useAppStore((s) => s.sidebarMode)
  const setSidebarMode = useAppStore((s) => s.setSidebarMode)
  const queryClient = useQueryClient()

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
      setOpenFolder(result.path, result.tree, result.truncated)
    }
  }, [setOpenFolder])

  const drawerTitle = mode === 'recents' ? 'Recents' : mode === 'folder' ? 'Folder' : 'Outline'
  const modeIndex = MODES.indexOf(mode)
  const railRoving = useRovingFocus({ orientation: 'vertical' })

  return (
    <div className="flex h-full shrink-0 border-r border-border-subtle">
      <div className="rail-column relative flex shrink-0 flex-col items-center border-r border-border-subtle bg-sidebar/40">
        <span
          aria-hidden
          className="rail-indicator pointer-events-none absolute left-0 w-0.5 rounded-r bg-primary"
          style={{
            height: 'var(--rail-indicator-height)',
            top: 0,
            transform: `translateY(${indicatorY(modeIndex)})`,
          }}
        />
        {/* oxlint-disable-next-line jsx-a11y/interactive-supports-focus -- per WAI-ARIA, focus rests on the active radio inside, not the radiogroup itself */}
        <div
          ref={railRoving.containerRef}
          role="radiogroup"
          aria-label="Sidebar mode"
          className="rail-mode-group flex flex-col"
          onKeyDown={railRoving.onKeyDown}
        >
          <RailModeIcon
            checked={mode === 'recents'}
            onSelect={() => setSidebarMode('recents')}
            label="Recents"
          >
            <Clock />
          </RailModeIcon>
          <RailModeIcon
            checked={mode === 'folder'}
            onSelect={() => setSidebarMode('folder')}
            label="Folder"
          >
            <Folder />
          </RailModeIcon>
          <RailModeIcon
            checked={mode === 'outline'}
            onSelect={() => setSidebarMode('outline')}
            label="Outline"
          >
            <List />
          </RailModeIcon>
        </div>
        <RailButton onClick={() => setCommandPaletteOpen(true)} label="Quick Open">
          <Search />
        </RailButton>
        <div className="flex-1" />
        <div aria-label="Workspace actions" className="contents">
          <RailButton onClick={() => void handleOpenFile()} label="Open File">
            <File />
          </RailButton>
          <RailButton onClick={() => void handleOpenFolder()} label="Open Folder">
            <FolderOpen />
          </RailButton>
          <RailButton onClick={() => setSettingsOpen(true)} label="Settings">
            <Settings />
          </RailButton>
        </div>
      </div>

      <div
        className="sidebar-drawer shrink-0 overflow-hidden"
        style={{
          width: sidebarOpen ? 'var(--sidebar-drawer-width)' : 0,
        }}
        aria-hidden={!sidebarOpen}
        inert={!sidebarOpen ? true : undefined}
      >
        <ShadcnSidebar
          collapsible="none"
          className="h-full border-none"
          style={{ width: 'var(--sidebar-drawer-width)' }}
        >
          <SidebarHeader className="sidebar-drawer-header">
            <div className="sidebar-drawer-title-row flex items-center">
              <span className="sidebar-drawer-title font-medium uppercase tracking-wider text-muted-foreground">
                {drawerTitle}
              </span>
            </div>
          </SidebarHeader>
          <SidebarContent key={mode} className="drawer-mode">
            {mode === 'recents' && <RecentsList />}
            {mode === 'folder' && openFolderPath && <FolderTree />}
            {mode === 'folder' && !openFolderPath && (
              <EmptyState
                size="sm"
                icon={FolderOpen}
                title="No folder open"
                hint={`Click Open Folder below, or drag a folder onto this window. Right-click a file to ${revealLabel.toLowerCase()}.`}
              />
            )}
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

function railClasses(active: boolean): string {
  return `rail-icon-btn size-(--rail-button-size) hover:bg-transparent ${
    active ? 'text-foreground' : 'text-muted-foreground hover:text-foreground'
  }`
}

function RailModeIcon({
  checked,
  onSelect,
  label,
  children,
}: {
  checked: boolean
  onSelect: () => void
  label: string
  children: React.ReactNode
}) {
  return (
    <Button
      variant="ghost"
      size="icon"
      // oxlint-disable-next-line jsx-a11y/prefer-tag-over-role -- the rail is a custom 36px-wide column of icon toggles; native radio inputs would break the layout
      role="radio"
      tabIndex={rovingTabIndex(checked)}
      aria-checked={checked}
      aria-label={label}
      title={label}
      className={railClasses(checked)}
      onClick={onSelect}
    >
      <span className="[&>svg]:size-(--rail-icon-size)">{children}</span>
    </Button>
  )
}

function RailButton({
  onClick,
  label,
  children,
}: {
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
      className={railClasses(false)}
      onClick={onClick}
    >
      <span className="[&>svg]:size-(--rail-icon-size)">{children}</span>
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
      <EmptyState
        size="sm"
        icon={List}
        title={hasActiveDoc ? 'No headings' : 'No document open'}
        hint={
          hasActiveDoc
            ? 'This document has no headings to show.'
            : 'Open a document to see its outline.'
        }
      />
    )
  }
  return (
    <SidebarGroup>
      <SidebarGroupContent>
        <ul className="flex flex-col gap-px px-1.5 py-1">
          {headings.map((h) => {
            const isActive = h.id === activeId
            return (
              <li key={h.id}>
                <a
                  href={`#${h.id}`}
                  data-active={isActive}
                  onClick={(e) => handleClick(e, h.id)}
                  className="outline-link block truncate rounded text-sidebar-foreground/75 hover:bg-sidebar-accent/60 hover:text-foreground"
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
