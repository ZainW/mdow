import { basename, dirname } from 'node:path'
import { memo, useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import { useGpuixRequired, type PublicInstance } from '@gpuix/react'
import type { OutlineEntry } from '../lib/markdown'
import { LABELS, SIDEBAR_MODES, type SidebarMode } from '../lib/prefs'
import { visibleRows, WORKSPACE_ERROR_COPY } from '../lib/workspace'
import { openDocumentAsync, setOverlay, setPrefs, toggleDirectory, useApp } from '../store'
import { METRICS, useUi } from './context'
import { activateOnEnter, Button, EmptyState, Icon, Label, withAlpha } from './primitives'
import { sendReader, useTopRow } from './reader-bus'
import type { IconName } from './icons'

const MODE_ICONS: Record<SidebarMode, IconName> = {
  recents: 'clock',
  folder: 'folder',
  outline: 'list',
}

export function Sidebar({ onOpenFolder }: { onOpenFolder: () => void }) {
  const { theme } = useUi()
  const mode = useApp((state) => state.prefs.sidebarMode)
  return (
    <div
      testId="sidebar"
      style={{
        display: 'flex',
        flexDirection: 'column',
        width: METRICS.sidebarWidth,
        flexShrink: 0,
        height: '100%',
        backgroundColor: theme.sidebar,
        borderRightWidth: 1,
        borderColor: theme.borderSubtle,
      }}
    >
      <ModeSwitcher mode={mode} />
      <div style={{ display: 'flex', flexDirection: 'column', flexGrow: 1, minHeight: 0 }}>
        {mode === 'recents' ? <Recents /> : null}
        {mode === 'folder' ? <FolderTree onOpenFolder={onOpenFolder} /> : null}
        {mode === 'outline' ? <Outline /> : null}
      </div>
      <SidebarFooter />
    </div>
  )
}

/** Sidebar header: 8/12 padding around a 28px rail, 45px with its bottom border. */
const HEADER_HEIGHT = 45

function ModeSwitcher({ mode }: { mode: SidebarMode }) {
  const { theme, scale } = useUi()
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 2,
        height: HEADER_HEIGHT,
        paddingLeft: 12,
        paddingRight: 12,
        paddingTop: 8,
        paddingBottom: 8,
        borderBottomWidth: 1,
        borderColor: theme.borderSubtle,
        flexShrink: 0,
        userSelect: 'none',
      }}
    >
      {SIDEBAR_MODES.map((item) => {
        const selected = item === mode
        const color = selected ? theme.foreground : theme.mutedForeground
        return (
          <div
            key={item}
            role="tab"
            aria-selected={selected}
            testId={`sidebar-mode-${item}`}
            tabIndex={0}
            onClick={() => setPrefs({ sidebarMode: item })}
            onKeyDown={activateOnEnter(() => setPrefs({ sidebarMode: item }))}
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              flexGrow: 1,
              minWidth: 0,
              gap: 4,
              height: scale.buttonHeight,
              paddingLeft: 4,
              paddingRight: 4,
              borderRadius: 6,
              cursor: 'pointer',
              backgroundColor: selected ? theme.sidebarAccent : undefined,
              hover: selected
                ? undefined
                : { backgroundColor: withAlpha(theme.sidebarAccent, 0.7) },
            }}
          >
            <Icon name={MODE_ICONS[item]} size={14} color={color} />
            <Label
              size={scale.controlFont - 1}
              color={color}
              weight={500}
              style={{ whiteSpace: 'nowrap', textOverflow: 'ellipsis' }}
            >
              {LABELS.sidebarMode[item]}
            </Label>
          </div>
        )
      })}
    </div>
  )
}

function SidebarFooter() {
  const { theme, scale } = useUi()
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        padding: 8,
        borderTopWidth: 1,
        borderColor: theme.borderSubtle,
        flexShrink: 0,
      }}
    >
      <div
        role="button"
        aria-label="Settings"
        tabIndex={0}
        onClick={() => setOverlay('settings')}
        onKeyDown={activateOnEnter(() => setOverlay('settings'))}
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          height: scale.buttonXsHeight + 4,
          paddingLeft: 8,
          paddingRight: 8,
          borderRadius: 6,
          cursor: 'pointer',
          hover: { backgroundColor: withAlpha(theme.muted, 0.5) },
          userSelect: 'none',
        }}
      >
        <Icon name="settings" size={14} />
        <Label size={scale.controlFont} weight={500} color={theme.mutedForeground}>
          Settings
        </Label>
      </div>
    </div>
  )
}

/** shadcn's SidebarGroup: 8px sides, 4px top and bottom. */
function SidebarGroup({ children, grow }: { children: ReactNode; grow?: boolean }) {
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        paddingLeft: 8,
        paddingRight: 8,
        paddingTop: 4,
        paddingBottom: 4,
        flexGrow: grow ? 1 : 0,
        minHeight: 0,
      }}
    >
      {children}
    </div>
  )
}

function GroupLabel({ children, detail }: { children: string; detail?: string }) {
  const { theme, scale } = useUi()
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: 8,
        height: 32,
        paddingLeft: 8,
        paddingRight: 8,
        flexShrink: 0,
        userSelect: 'none',
      }}
    >
      <Label size={scale.controlFont} color={withAlpha(theme.foreground, 0.7)}>
        {children}
      </Label>
      {detail ? (
        <Label
          size={scale.controlFont}
          color={withAlpha(theme.mutedForeground, 0.6)}
          style={{ whiteSpace: 'nowrap', textOverflow: 'ellipsis', flexShrink: 1, minWidth: 0 }}
        >
          {detail}
        </Label>
      ) : null}
    </div>
  )
}

/** The 2px accent bar desktop draws beside the current file (`.tree-file-active`). */
function ActiveBar() {
  const { theme } = useUi()
  return (
    <div
      style={{
        position: 'absolute',
        left: -1,
        top: 4,
        bottom: 4,
        width: 2,
        borderRadius: 1,
        backgroundColor: theme.accent,
      }}
    />
  )
}

function ScrollArea({ children }: { children: ReactNode }) {
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        flexGrow: 1,
        minHeight: 0,
        overflowY: 'scroll',
        gap: 1,
      }}
    >
      {children}
    </div>
  )
}

/** Desktop's `parentDir`: the last two folders above the file. */
function parentDir(path: string) {
  const parts = dirname(path).split(/[/\\]/).filter(Boolean)
  return parts.slice(-2).join('/')
}

function Recents() {
  const { theme, scale } = useUi()
  const recents = useApp((state) => state.recents)
  const activePath = useApp((state) => state.activePath)
  if (recents.length === 0) {
    return (
      <EmptyState
        size="sm"
        icon="clock"
        title="No recents yet"
        body="Files you open will appear here."
      />
    )
  }
  return (
    <SidebarGroup grow>
      <GroupLabel>Recents</GroupLabel>
      <ScrollArea>
        {recents.map((path) => {
          const selected = path === activePath
          const dir = parentDir(path)
          return (
            <div
              key={path}
              role="button"
              tabIndex={0}
              onClick={() => void openDocumentAsync(path)}
              onKeyDown={activateOnEnter(() => void openDocumentAsync(path))}
              style={{
                position: 'relative',
                display: 'flex',
                flexDirection: 'column',
                flexShrink: 0,
                paddingTop: 6,
                paddingBottom: 6,
                paddingLeft: 8,
                paddingRight: 8,
                borderRadius: 6,
                cursor: 'pointer',
                backgroundColor: selected ? theme.sidebarAccent : undefined,
                hover: { backgroundColor: theme.sidebarAccent },
                userSelect: 'none',
              }}
            >
              {selected ? <ActiveBar /> : null}
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, minWidth: 0 }}>
                <Icon name="file-text" size={14} color={withAlpha(theme.foreground, 0.4)} />
                <Label
                  size={scale.controlFont}
                  weight={selected ? 500 : 400}
                  style={{ whiteSpace: 'nowrap', textOverflow: 'ellipsis', lineHeight: 16 }}
                >
                  {basename(path)}
                </Label>
              </div>
              {dir ? (
                <Label
                  size={scale.controlXsFont}
                  color={withAlpha(theme.mutedForeground, 0.9)}
                  style={{
                    whiteSpace: 'nowrap',
                    textOverflow: 'ellipsis',
                    paddingLeft: 20,
                    lineHeight: 13,
                  }}
                >
                  {dir}
                </Label>
              ) : null}
            </div>
          )
        })}
      </ScrollArea>
    </SidebarGroup>
  )
}

function TreeRow({
  icon,
  title,
  depth,
  selected,
  onClick,
}: {
  icon: IconName
  title: string
  depth: number
  selected: boolean
  onClick: () => void
}) {
  const { theme, scale } = useUi()
  return (
    <div
      role="button"
      tabIndex={0}
      onClick={onClick}
      onKeyDown={activateOnEnter(onClick)}
      style={{
        position: 'relative',
        display: 'flex',
        alignItems: 'center',
        flexShrink: 0,
        gap: 6,
        height: scale.buttonHeight,
        paddingLeft: 8 + depth * 12,
        paddingRight: 8,
        borderRadius: 6,
        cursor: 'pointer',
        backgroundColor: selected ? theme.sidebarAccent : undefined,
        hover: { backgroundColor: theme.sidebarAccent },
        userSelect: 'none',
      }}
    >
      {selected ? <ActiveBar /> : null}
      <Icon name={icon} size={14} color={withAlpha(theme.foreground, selected ? 0.7 : 0.4)} />
      <Label
        size={scale.controlFont}
        weight={selected ? 500 : 400}
        color={selected ? theme.foreground : withAlpha(theme.foreground, 0.85)}
        style={{ whiteSpace: 'nowrap', textOverflow: 'ellipsis' }}
      >
        {title}
      </Label>
    </div>
  )
}

function FolderTree({ onOpenFolder }: { onOpenFolder: () => void }) {
  const workspace = useApp((state) => state.workspace)
  const activePath = useApp((state) => state.activePath)
  const { theme, scale } = useUi()
  const rows = useMemo(
    () => (workspace?.scan.ok ? visibleRows(workspace.scan.root, workspace.collapsed) : []),
    [workspace],
  )
  if (!workspace) {
    return (
      <EmptyState
        size="sm"
        icon="folder-open"
        title="No folder open"
        body="Use the app menu, keyboard shortcut, or drag a folder onto this window."
      />
    )
  }
  if (!workspace.scan.ok) {
    const copy = WORKSPACE_ERROR_COPY[workspace.scan.error]
    return (
      <EmptyState
        size="sm"
        icon="alert-circle"
        title={copy.title}
        body={copy.body}
        detail={workspace.folder}
      >
        <Button icon="folder-open" onClick={onOpenFolder} small>
          Open Folder
        </Button>
      </EmptyState>
    )
  }
  return (
    <SidebarGroup grow>
      <GroupLabel detail={workspace.scan.root.name}>Folder</GroupLabel>
      {rows.length === 0 ? (
        <div style={{ paddingLeft: 12, paddingRight: 12, paddingTop: 16, alignItems: 'center' }}>
          <Label
            size={scale.controlFont}
            color={theme.mutedForeground}
            style={{ textAlign: 'center' }}
          >
            No Markdown or HTML files in this folder.
          </Label>
        </div>
      ) : (
        <ScrollArea>
          {rows.map((row) => (
            <TreeRow
              key={row.path}
              icon={
                row.kind === 'directory'
                  ? row.expanded
                    ? 'chevron-down'
                    : 'chevron-right'
                  : 'file-text'
              }
              title={row.name}
              depth={row.depth}
              selected={row.path === activePath}
              onClick={() =>
                row.kind === 'directory'
                  ? toggleDirectory(row.path)
                  : void openDocumentAsync(row.path)
              }
            />
          ))}
        </ScrollArea>
      )}
    </SidebarGroup>
  )
}

function Outline() {
  const tab = useApp((state) => state.tabs.find((item) => item.path === state.activePath))
  const outline = tab?.document?.ok ? tab.document.parsed.outline : []
  if (outline.length === 0) {
    return (
      <EmptyState
        size="sm"
        icon="list"
        title={tab ? 'No headings' : 'No document open'}
        body={
          tab ? 'This document has no headings to show.' : 'Open a document to see its outline.'
        }
      />
    )
  }
  return <OutlineList outline={outline} />
}

/** Outline rows mounted at once. A 3MB document has ~16k headings, so the outline is windowed. */
const OUTLINE_WINDOW = 160
/** A 24px row plus the 1px gap below it. */
const OUTLINE_ROW = 25

function OutlineList({ outline }: { outline: OutlineEntry[] }) {
  const renderer = useGpuixRequired()
  const listRef = useRef<PublicInstance | null>(null)
  const topRow = useTopRow()
  const [start, setStart] = useState(0)
  const shown = useRef<[number, number]>([0, 0])
  const pendingScroll = useRef<number | null>(null)
  // The current section is the last heading at or above the reading line.
  let active = -1
  for (let i = 0; i < outline.length && outline[i]!.blockIndex <= topRow; i++) active = i
  const windowStart = Math.min(start, Math.max(0, outline.length - 1))
  const end = Math.min(outline.length, windowStart + OUTLINE_WINDOW)

  // Follow the reader: keep the current section in view without fighting a user scrolling here.
  useEffect(() => {
    const id = listRef.current?.id
    if (active < 0 || id === undefined) return
    const [first, last] = shown.current
    if (active >= first && active < last - 1) return
    const target = Math.max(0, active - 3)
    if (target >= windowStart && target < end) {
      renderer.scrollToItem?.(id, target, 0)
      return
    }
    // Outside the rendered window: move the window first, then scroll once it has committed
    // (scrolling now would target a row the list doesn't have yet and stop short).
    pendingScroll.current = target
    setStart(Math.max(0, target - OUTLINE_WINDOW / 4))
  }, [active, renderer, windowStart, end])
  useEffect(() => {
    const id = listRef.current?.id
    const target = pendingScroll.current
    if (target === null || id === undefined) return
    pendingScroll.current = null
    renderer.scrollToItem?.(id, target, 0)
  }, [windowStart, renderer])

  const rows = []
  for (let index = windowStart; index < end; index++) {
    const entry = outline[index]!
    rows.push(
      <OutlineRow
        key={entry.slug}
        entry={entry}
        indent={Math.max(0, entry.level - 1)}
        active={index === active}
      />,
    )
  }
  return (
    // Desktop: SidebarGroup (8/4) around the list's own 6/4 padding, so rows sit at x=14. The
    // virtual-list ignores its own padding, so the frame carries it; the list already insets its
    // first row by 4px, hence 4 on top.
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        flexGrow: 1,
        minHeight: 0,
        paddingLeft: 14,
        paddingRight: 14,
        paddingTop: 4,
      }}
    >
      <virtual-list
        ref={listRef}
        testId="outline"
        itemCount={outline.length}
        windowStart={windowStart}
        estimatedItemHeight={OUTLINE_ROW}
        overdraw={200}
        onVisibleRange={(event) => {
          const first = Math.floor(event.startIndex ?? 0)
          const last = Math.ceil(event.endIndex ?? first)
          shown.current = [first, last]
          if (first < windowStart + OUTLINE_WINDOW / 8 || last > end - OUTLINE_WINDOW / 8) {
            const next = Math.max(0, first - OUTLINE_WINDOW / 4)
            if (next !== windowStart) setStart(next)
          }
        }}
        style={{ flexGrow: 1, minHeight: 0 }}
      >
        {rows}
      </virtual-list>
    </div>
  )
}

const OutlineRow = memo(function OutlineRow({
  entry,
  indent,
  active,
}: {
  entry: OutlineEntry
  indent: number
  active: boolean
}) {
  const { theme, scale } = useUi()
  const go = () => sendReader({ type: 'block', index: entry.blockIndex })
  return (
    <div style={{ paddingBottom: 1 }}>
      <div
        role="button"
        aria-current={active ? 'location' : undefined}
        tabIndex={0}
        onClick={go}
        onKeyDown={activateOnEnter(go)}
        style={{
          paddingTop: 4,
          paddingBottom: 4,
          paddingLeft: 6 + indent * 10,
          paddingRight: 6,
          borderRadius: 4,
          cursor: 'pointer',
          backgroundColor: active ? theme.sidebarAccent : undefined,
          hover: active ? undefined : { backgroundColor: withAlpha(theme.sidebarAccent, 0.6) },
          userSelect: 'none',
        }}
      >
        <Label
          size={scale.controlFont}
          color={active ? theme.foreground : withAlpha(theme.foreground, 0.75)}
          style={{ whiteSpace: 'nowrap', textOverflow: 'ellipsis', lineHeight: 16 }}
        >
          {entry.text}
        </Label>
      </div>
    </div>
  )
})
