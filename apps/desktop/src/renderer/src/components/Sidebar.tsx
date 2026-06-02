import { lazy, Suspense, useCallback } from 'react'
import { useAppStore, type SidebarMode } from '../store/app-store'
import { RecentsList } from './RecentsList'
import { Button } from './ui/button'
import {
  Sidebar as ShadcnSidebar,
  SidebarContent,
  SidebarHeader,
  SidebarGroup,
  SidebarGroupContent,
} from './ui/sidebar'
import { Clock, Folder, FolderOpen, List } from 'lucide-react'
import type { DocHeading } from '../lib/markdown'
import { EmptyState } from './EmptyState'
import { rovingTabIndex, useRovingFocus } from '../hooks/useRovingFocus'
import { isMac } from '../lib/utils'

const MODES: SidebarMode[] = ['recents', 'folder', 'outline']
const MODE_CONFIG: Record<SidebarMode, { label: string; Icon: typeof Clock }> = {
  recents: { label: 'Recents', Icon: Clock },
  folder: { label: 'Folder', Icon: Folder },
  outline: { label: 'Outline', Icon: List },
}
const revealLabel = isMac ? 'Reveal in Finder' : 'Show in Folder'
const FolderTree = lazy(() => import('./FolderTree').then((mod) => ({ default: mod.FolderTree })))
type SidebarModeRoving = ReturnType<typeof useRovingFocus<HTMLDivElement>>

export function Sidebar() {
  const sidebarOpen = useAppStore((s) => s.sidebarOpen)
  const docHeadings = useAppStore((s) => s.docHeadings)
  const activeHeadingId = useAppStore((s) => s.activeHeadingId)
  const hasOpenTab = useAppStore((s) => s.tabs.length > 0)
  const openFolderPath = useAppStore((s) => s.openFolderPath)
  const mode = useAppStore((s) => s.sidebarMode)
  const setSidebarMode = useAppStore((s) => s.setSidebarMode)
  const modeRoving = useRovingFocus({ orientation: 'horizontal' })

  return (
    <aside
      aria-label="Sidebar"
      className="sidebar-drawer shrink-0 overflow-hidden border-r border-border-subtle"
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
        <SidebarHeader className="sidebar-drawer-header border-b border-border-subtle">
          <SidebarModeTabs mode={mode} onModeChange={setSidebarMode} roving={modeRoving} />
        </SidebarHeader>
        <SidebarContent key={mode} className="drawer-mode">
          {mode === 'recents' && <RecentsList />}
          {mode === 'folder' && openFolderPath && (
            <Suspense fallback={<FolderTreeSkeleton />}>
              <FolderTree />
            </Suspense>
          )}
          {mode === 'folder' && !openFolderPath && (
            <EmptyState
              size="sm"
              icon={FolderOpen}
              title="No folder open"
              hint={`Use the app menu, keyboard shortcut, or drag a folder onto this window. Right-click a file to ${revealLabel.toLowerCase()}.`}
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
    </aside>
  )
}

function SidebarModeTabs({
  mode,
  onModeChange,
  roving,
}: {
  mode: SidebarMode
  onModeChange: (mode: SidebarMode) => void
  roving: SidebarModeRoving
}) {
  return (
    // oxlint-disable-next-line jsx-a11y/interactive-supports-focus -- per WAI-ARIA, focus rests on the active radio inside, not the radiogroup itself
    <div
      ref={roving.containerRef}
      role="radiogroup"
      aria-label="Sidebar mode"
      className="grid grid-cols-3 gap-1"
    >
      {MODES.map((item) => (
        <SidebarModeTab
          key={item}
          mode={item}
          checked={mode === item}
          onSelect={() => onModeChange(item)}
          onKeyDown={roving.onKeyDown}
        />
      ))}
    </div>
  )
}

function SidebarModeTab({
  mode,
  checked,
  onSelect,
  onKeyDown,
}: {
  mode: SidebarMode
  checked: boolean
  onSelect: () => void
  onKeyDown: React.KeyboardEventHandler<HTMLElement>
}) {
  const { label, Icon } = MODE_CONFIG[mode]

  return (
    <Button
      variant="ghost"
      size="sm"
      // oxlint-disable-next-line jsx-a11y/prefer-tag-over-role -- custom radio buttons preserve the compact tab layout while exposing radiogroup semantics
      role="radio"
      tabIndex={rovingTabIndex(checked)}
      aria-checked={checked}
      aria-label={label}
      title={label}
      className={`h-7 min-w-0 justify-center gap-1.5 px-1.5 text-xs ${
        checked
          ? 'bg-sidebar-accent text-sidebar-accent-foreground'
          : 'text-muted-foreground hover:bg-sidebar-accent/70 hover:text-foreground'
      }`}
      onClick={onSelect}
      onKeyDown={onKeyDown}
    >
      <Icon className="size-3.5 shrink-0" aria-hidden />
      <span className="truncate">{label}</span>
    </Button>
  )
}

function FolderTreeSkeleton() {
  return (
    <div aria-hidden className="flex flex-col gap-2 p-3">
      <div className="h-3 w-24 rounded bg-muted" />
      <div className="h-3 w-36 rounded bg-muted/80" />
      <div className="ml-3 h-3 w-28 rounded bg-muted/70" />
      <div className="ml-3 h-3 w-32 rounded bg-muted/70" />
    </div>
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
