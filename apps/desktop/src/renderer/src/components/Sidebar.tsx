import { lazy, Suspense, useCallback, useEffect, useMemo, useRef } from 'react'
import { useVirtualizer } from '@tanstack/react-virtual'
import { useAppStore, selectActiveTab, type SidebarMode } from '../store/app-store'
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
import type { TreeNode } from '../../../shared/types'
import { EmptyState } from './EmptyState'
import { SegmentedControl, type SegmentedOption } from './SegmentedControl'
import { findMarkdownScroller, scrollToTarget } from '../lib/scroll-to'
import { basename } from '../lib/path-utils'
import { cn, formatShortcut, isMac } from '../lib/utils'
import { useClearRecents, useRecents } from '../hooks/useRecents'
import { useOpenFolderDialog } from '../hooks/useOpenFolderDialog'

const MODE_OPTIONS: readonly SegmentedOption<SidebarMode>[] = [
  { value: 'recents', label: 'Recents', Icon: Clock },
  { value: 'folder', label: 'Folder', Icon: Folder },
  { value: 'outline', label: 'Outline', Icon: List },
]
const revealLabel = isMac ? 'Reveal in Finder' : 'Show in Folder'
const FolderTree = lazy(() => import('./FolderTree').then((mod) => ({ default: mod.FolderTree })))
const OUTLINE_ROW_ESTIMATE = 26
// Width of one outline indent column (each holds a guide line), in px.
const OUTLINE_INDENT = 12

export function countTreeFiles(nodes: readonly TreeNode[]): number {
  let count = 0
  for (const node of nodes) {
    count += node.isDirectory ? countTreeFiles(node.children ?? []) : 1
  }
  return count
}

function formatCount(count: number, noun: string, { atLeast = false } = {}): string {
  return `${count.toLocaleString()}${atLeast ? '+' : ''} ${noun}${count === 1 ? '' : 's'}`
}

export function Sidebar() {
  const sidebarOpen = useAppStore((s) => s.sidebarOpen)
  const docHeadings = useAppStore((s) => s.docHeadings)
  const activeHeadingId = useAppStore((s) => s.activeHeadingId)
  const hasOpenTab = useAppStore((s) => s.tabs.length > 0)
  const openFolderPath = useAppStore((s) => s.openFolderPath)
  const mode = useAppStore((s) => s.sidebarMode)
  const setSidebarMode = useAppStore((s) => s.setSidebarMode)
  const setSettingsOpen = useAppStore((s) => s.setSettingsOpen)

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
        <SidebarHeader className="sidebar-drawer-header gap-0">
          <SegmentedControl
            label="Sidebar mode"
            value={mode}
            options={MODE_OPTIONS}
            onChange={setSidebarMode}
            // Three modes share ~220px: tighter than the Settings segments so labels never clip.
            segmentClassName="gap-1 px-1 text-[11.5px] [&_svg]:size-[13px]"
          />
        </SidebarHeader>
        <SidebarSectionHeader mode={mode} />
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
            aria-keyshortcuts={isMac ? 'Meta+,' : 'Control+,'}
            title="Settings"
            className="w-full justify-start gap-2 text-muted-foreground hover:text-foreground"
            onClick={() => setSettingsOpen(true)}
          >
            <Settings className="size-3.5 shrink-0" aria-hidden />
            <span className="flex-1 text-left">Settings</span>
            <kbd className="sidebar-kbd" aria-hidden>
              {formatShortcut(',')}
            </kbd>
          </Button>
        </SidebarFooter>
      </ShadcnSidebar>
    </aside>
  )
}

/**
 * A fixed-height row under the mode switcher that every mode fills (name, count,
 * action), so the list below starts at the same y whichever mode is showing.
 */
function SidebarSectionHeader({ mode }: { mode: SidebarMode }) {
  return (
    <div
      data-testid="sidebar-section-header"
      className="flex h-[30px] shrink-0 items-center gap-1.5 pr-2 pl-3.5"
    >
      {mode === 'recents' && <RecentsHeader />}
      {mode === 'folder' && <FolderHeader />}
      {mode === 'outline' && <OutlineHeader />}
    </div>
  )
}

function SectionTitle({ children, muted }: { children: React.ReactNode; muted?: boolean }) {
  return (
    <h2
      className={
        muted
          ? 'min-w-0 flex-1 truncate text-xs font-medium text-muted-foreground'
          : 'min-w-0 flex-1 truncate text-xs font-semibold text-foreground'
      }
    >
      {children}
    </h2>
  )
}

function SectionCount({ children }: { children: React.ReactNode }) {
  return (
    <span className="shrink-0 text-[10.5px] font-medium text-muted-foreground tabular-nums">
      {children}
    </span>
  )
}

function RecentsHeader() {
  const { data: recents = [] } = useRecents()
  const clearRecents = useClearRecents()
  return (
    <>
      <SectionTitle>Recent files</SectionTitle>
      {recents.length > 0 && (
        <Button
          type="button"
          variant="ghost"
          size="xs"
          className="h-6 px-1.5 text-[10.5px] text-muted-foreground hover:text-foreground"
          onClick={() => void clearRecents()}
          aria-label="Clear recent files"
          title="Clear recent files"
        >
          Clear
        </Button>
      )}
    </>
  )
}

function FolderHeader() {
  const openFolderPath = useAppStore((s) => s.openFolderPath)
  const folderTree = useAppStore((s) => s.folderTree)
  const truncated = useAppStore((s) => s.folderTreeTruncated)
  const openFolderDialog = useOpenFolderDialog()
  const fileCount = useMemo(() => countTreeFiles(folderTree), [folderTree])

  return (
    <>
      {openFolderPath ? (
        <>
          <SectionTitle>
            <span title={openFolderPath}>{basename(openFolderPath)}</span>
          </SectionTitle>
          <SectionCount>{formatCount(fileCount, 'file', { atLeast: truncated })}</SectionCount>
        </>
      ) : (
        <SectionTitle muted>No folder</SectionTitle>
      )}
      <Button
        type="button"
        variant="ghost"
        size="icon-xs"
        className="size-6 text-muted-foreground hover:text-foreground"
        aria-label="Open folder"
        title={`Open folder (${formatShortcut('O', { shift: true })})`}
        onClick={() => void openFolderDialog()}
      >
        <FolderOpen className="size-3.5" aria-hidden />
      </Button>
    </>
  )
}

function OutlineHeader() {
  const docHeadings = useAppStore((s) => s.docHeadings)
  const activeTab = useAppStore(selectActiveTab)
  if (!activeTab) return <SectionTitle muted>No document</SectionTitle>
  const title =
    docHeadings.find((h) => h.level === 1)?.text ?? docHeadings[0]?.text ?? basename(activeTab.path)
  return (
    <>
      <SectionTitle>
        <span title={title}>{title}</span>
      </SectionTitle>
      <SectionCount>{formatCount(docHeadings.length, 'heading')}</SectionCount>
    </>
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
  // Indent relative to the shallowest heading, so a document without an h1
  // doesn't start every row one level in.
  const minLevel = useMemo(() => headings.reduce((min, h) => Math.min(min, h.level), 6), [headings])

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
      <SidebarGroup className="pt-0">
        <SidebarGroupContent>
          <ul className="relative px-1.5 py-1" style={{ height: virtualizer.getTotalSize() }}>
            {virtualizer.getVirtualItems().map((row) => {
              const h = headings[row.index]
              const isActive = row.index === activeIndex
              const depth = Math.max(0, h.level - minLevel)
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
                    className={cn(
                      'outline-link flex items-stretch rounded hover:bg-sidebar-accent/60 hover:text-foreground',
                      depth <= 1 ? 'text-sidebar-foreground/90' : 'text-muted-foreground',
                    )}
                    title={h.text}
                  >
                    {Array.from({ length: depth }, (_, i) => (
                      <span
                        // oxlint-disable-next-line react/no-array-index-key -- guides are positional
                        key={i}
                        aria-hidden
                        data-outline-guide=""
                        className="outline-guide shrink-0"
                        style={{ width: OUTLINE_INDENT }}
                      />
                    ))}
                    <span className="min-w-0 truncate">{h.text}</span>
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
