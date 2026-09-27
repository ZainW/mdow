import { lazy, Suspense, useCallback, useEffect, useMemo, useRef } from 'react'
import { useVirtualizer } from '@tanstack/react-virtual'
import { useAppStore, type SidebarMode } from '../store/app-store'
import { RecentsList } from './RecentsList'
import { Button } from './ui/button'
import {
  Sidebar as ShadcnSidebar,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarGroup,
  SidebarGroupContent,
} from './ui/sidebar'
import { Clock, Folder, FolderOpen, List, Settings } from 'lucide-react'
import type { DocHeading } from '../lib/markdown'
import { EmptyState } from './EmptyState'
import { findMarkdownScroller, scrollToTarget } from '../lib/scroll-to'
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
const OUTLINE_ROW_ESTIMATE = 26
type SidebarModeRoving = ReturnType<typeof useRovingFocus<HTMLDivElement>>

export function Sidebar() {
  const sidebarOpen = useAppStore((s) => s.sidebarOpen)
  const docHeadings = useAppStore((s) => s.docHeadings)
  const activeHeadingId = useAppStore((s) => s.activeHeadingId)
  const hasOpenTab = useAppStore((s) => s.tabs.length > 0)
  const openFolderPath = useAppStore((s) => s.openFolderPath)
  const mode = useAppStore((s) => s.sidebarMode)
  const setSidebarMode = useAppStore((s) => s.setSidebarMode)
  const setSettingsOpen = useAppStore((s) => s.setSettingsOpen)
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
        <SidebarContent key={mode}>
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
        <SidebarFooter className="border-t border-border-subtle p-2">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            aria-label="Settings"
            title="Settings"
            className="w-full justify-start gap-2 text-muted-foreground hover:text-foreground"
            onClick={() => setSettingsOpen(true)}
          >
            <Settings className="size-3.5 shrink-0" aria-hidden />
            <span>Settings</span>
          </Button>
        </SidebarFooter>
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
      className="flex gap-0.5"
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
      className={`h-7 min-w-0 flex-auto justify-center gap-1 px-1 text-[length:var(--sidebar-title-size)] ${
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
  const scrollRef = useRef<HTMLDivElement>(null)
  // Long documents have tens of thousands of headings; only the visible rows are rendered.
  // oxlint-disable-next-line react/incompatible-library -- the outline is not memoized by the compiler; the virtualizer drives its re-renders.
  const virtualizer = useVirtualizer({
    count: headings.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => OUTLINE_ROW_ESTIMATE,
    overscan: 12,
  })
  const activeIndex = useMemo(
    () => (activeId ? headings.findIndex((h) => h.id === activeId) : -1),
    [headings, activeId],
  )

  // Keep the reader's current heading in view as they scroll the document.
  useEffect(() => {
    if (activeIndex >= 0) virtualizer.scrollToIndex(activeIndex, { align: 'auto' })
  }, [activeIndex, virtualizer])

  const handleClick = useCallback((e: React.MouseEvent<HTMLAnchorElement>, id: string) => {
    e.preventDefault()
    const el = document.getElementById(id)
    const scroller = el && findMarkdownScroller(el)
    if (!el || !scroller) return
    scrollToTarget(scroller, el, { smooth: true })
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
    <div ref={scrollRef} className="no-scrollbar min-h-0 flex-1 overflow-y-auto">
      <SidebarGroup>
        <SidebarGroupContent>
          <ul className="relative px-1.5 py-1" style={{ height: virtualizer.getTotalSize() }}>
            {virtualizer.getVirtualItems().map((row) => {
              const h = headings[row.index]
              const isActive = row.index === activeIndex
              return (
                <li
                  key={h.id}
                  data-index={row.index}
                  ref={virtualizer.measureElement}
                  className="absolute inset-x-1.5 top-0 pb-px"
                  style={{ transform: `translateY(${row.start}px)` }}
                >
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
    </div>
  )
}
