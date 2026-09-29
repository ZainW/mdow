import { basename, dirname } from 'node:path'
import { memo, useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import { useGpuixRequired, type PublicInstance } from '@gpuix/react'
import type { OutlineEntry } from '../lib/markdown'
import { LABELS, SIDEBAR_MODES, type SidebarMode } from '../lib/prefs'
import { visibleRows, WORKSPACE_ERROR_COPY } from '../lib/workspace'
import { openDocument, setOverlay, setPrefs, toggleDirectory, useApp } from '../store'
import { METRICS, useUi } from './context'
import { activateOnEnter, Button, EmptyState, Icon, Label } from './primitives'
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
        borderColor: theme.border,
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

function ModeSwitcher({ mode }: { mode: SidebarMode }) {
  const { theme, scale } = useUi()
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 2,
        height: METRICS.chromeRowHeight,
        paddingLeft: 6,
        paddingRight: 6,
        borderBottomWidth: 1,
        borderColor: theme.border,
        flexShrink: 0,
        userSelect: 'none',
      }}
    >
      {SIDEBAR_MODES.map((item) => {
        const selected = item === mode
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
              gap: 5,
              height: scale.buttonHeight + 4,
              paddingLeft: 7,
              paddingRight: 8,
              borderRadius: 6,
              cursor: 'pointer',
              backgroundColor: selected ? theme.sidebarAccent : undefined,
              hover: { backgroundColor: theme.sidebarAccent },
            }}
          >
            <Icon
              name={MODE_ICONS[item]}
              color={selected ? theme.foreground : theme.mutedForeground}
            />
            <Label
              size={scale.controlFont + 0.5}
              color={selected ? theme.foreground : theme.mutedForeground}
              weight={selected ? 500 : 400}
              style={{ whiteSpace: 'nowrap' }}
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
  const { theme } = useUi()
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        height: METRICS.chromeRowHeight,
        paddingLeft: 6,
        paddingRight: 6,
        borderTopWidth: 1,
        borderColor: theme.border,
        flexShrink: 0,
      }}
    >
      <div
        role="button"
        tabIndex={0}
        onClick={() => setOverlay('settings')}
        onKeyDown={activateOnEnter(() => setOverlay('settings'))}
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          height: 30,
          paddingLeft: 8,
          paddingRight: 10,
          borderRadius: 6,
          cursor: 'pointer',
          hover: { backgroundColor: theme.sidebarAccent },
          userSelect: 'none',
        }}
      >
        <Icon name="settings" />
        <Label color={theme.mutedForeground}>Settings</Label>
      </div>
    </div>
  )
}

function SidebarRow({
  icon,
  title,
  detail,
  depth = 0,
  selected,
  onClick,
  testId,
}: {
  icon: IconName
  title: string
  detail?: string
  depth?: number
  selected?: boolean
  onClick: () => void
  testId?: string
}) {
  const { theme, scale } = useUi()
  return (
    <div
      role="button"
      testId={testId}
      tabIndex={0}
      onClick={onClick}
      onKeyDown={activateOnEnter(onClick)}
      style={{
        display: 'flex',
        alignItems: detail ? 'flex-start' : 'center',
        gap: 8,
        paddingTop: detail ? 6 : 5,
        paddingBottom: detail ? 6 : 5,
        paddingLeft: 10 + depth * 14,
        paddingRight: 10,
        borderRadius: 6,
        cursor: 'pointer',
        backgroundColor: selected ? theme.sidebarAccent : undefined,
        hover: { backgroundColor: theme.sidebarAccent },
        userSelect: 'none',
      }}
    >
      <div style={{ paddingTop: detail ? 2 : 0 }}>
        <Icon name={icon} size={14} color={selected ? theme.foreground : theme.mutedForeground} />
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 2, minWidth: 0, flexGrow: 1 }}>
        <Label
          color={theme.foreground}
          weight={selected ? 500 : 400}
          style={{ whiteSpace: 'nowrap', textOverflow: 'ellipsis' }}
        >
          {title}
        </Label>
        {detail ? (
          <Label
            size={scale.controlXsFont + 1}
            color={theme.mutedForeground}
            style={{ whiteSpace: 'nowrap', textOverflow: 'ellipsis' }}
          >
            {detail}
          </Label>
        ) : null}
      </div>
    </div>
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
        padding: 6,
        gap: 1,
      }}
    >
      {children}
    </div>
  )
}

function Recents() {
  const recents = useApp((state) => state.recents)
  const activePath = useApp((state) => state.activePath)
  if (recents.length === 0) {
    return (
      <div style={{ display: 'flex', justifyContent: 'center', paddingTop: 16 }}>
        <EmptyState icon="clock" title="No recents yet" body="Files you open will appear here." />
      </div>
    )
  }
  return (
    <ScrollArea>
      {recents.map((path) => (
        <SidebarRow
          key={path}
          icon="file"
          title={basename(path)}
          detail={basename(dirname(path))}
          selected={path === activePath}
          onClick={() => openDocument(path)}
        />
      ))}
    </ScrollArea>
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
      <div
        style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', paddingTop: 16 }}
      >
        <EmptyState
          icon="folder"
          title="No folder open"
          body="Open a folder to browse its Markdown files."
        >
          <Button icon="folder-open" onClick={onOpenFolder} small>
            Open Folder
          </Button>
        </EmptyState>
      </div>
    )
  }
  if (!workspace.scan.ok) {
    const copy = WORKSPACE_ERROR_COPY[workspace.scan.error]
    return (
      <div style={{ display: 'flex', justifyContent: 'center', paddingTop: 16 }}>
        <EmptyState
          icon="alert-circle"
          title={copy.title}
          body={copy.body}
          detail={workspace.folder}
        >
          <Button icon="folder-open" onClick={onOpenFolder} small>
            Open Folder
          </Button>
        </EmptyState>
      </div>
    )
  }
  return (
    <div style={{ display: 'flex', flexDirection: 'column', flexGrow: 1, minHeight: 0 }}>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 6,
          paddingLeft: 16,
          paddingRight: 12,
          paddingTop: 10,
          paddingBottom: 4,
        }}
      >
        <Label
          size={scale.controlXsFont + 1}
          weight={600}
          color={theme.mutedForeground}
          style={{ whiteSpace: 'nowrap', textOverflow: 'ellipsis' }}
        >
          {workspace.scan.root.name.toUpperCase()}
        </Label>
      </div>
      {rows.length === 0 ? (
        <div style={{ padding: 16 }}>
          <Label color={theme.mutedForeground}>No Markdown or HTML files in this folder.</Label>
        </div>
      ) : (
        <ScrollArea>
          {rows.map((row) => (
            <SidebarRow
              key={row.path}
              icon={
                row.kind === 'directory'
                  ? row.expanded
                    ? 'chevron-down'
                    : 'chevron-right'
                  : 'file'
              }
              title={row.name}
              depth={row.depth}
              selected={row.path === activePath}
              onClick={() =>
                row.kind === 'directory' ? toggleDirectory(row.path) : openDocument(row.path)
              }
            />
          ))}
        </ScrollArea>
      )}
    </div>
  )
}

function Outline() {
  const tab = useApp((state) => state.tabs.find((item) => item.path === state.activePath))
  const outline = tab?.document.ok ? tab.document.parsed.outline : []
  if (outline.length === 0) {
    return (
      <div style={{ display: 'flex', justifyContent: 'center', paddingTop: 16 }}>
        <EmptyState
          icon="list"
          title="No headings"
          body={tab ? 'This document has no headings.' : 'Open a document to see its outline.'}
        />
      </div>
    )
  }
  return <OutlineList outline={outline} />
}

/** Outline rows mounted at once. A 3MB document has ~16k headings, so the outline is windowed. */
const OUTLINE_WINDOW = 160
const OUTLINE_ROW = 30

function OutlineList({ outline }: { outline: OutlineEntry[] }) {
  const renderer = useGpuixRequired()
  const listRef = useRef<PublicInstance | null>(null)
  const topRow = useTopRow()
  const minLevel = useMemo(() => Math.min(...outline.map((entry) => entry.level)), [outline])
  const [start, setStart] = useState(0)
  const shown = useRef<[number, number]>([0, 0])
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
    setStart((current) =>
      active >= current && active < current + OUTLINE_WINDOW
        ? current
        : Math.max(0, active - OUTLINE_WINDOW / 4),
    )
    renderer.scrollToItem?.(id, Math.max(0, active - 3), 0)
  }, [active, renderer])

  const rows = []
  for (let index = windowStart; index < end; index++) {
    const entry = outline[index]!
    rows.push(
      <OutlineRow
        key={entry.slug}
        entry={entry}
        indent={entry.level - minLevel}
        active={index === active}
      />,
    )
  }
  return (
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
      style={{ flexGrow: 1, minHeight: 0, paddingLeft: 6, paddingRight: 6, paddingTop: 6 }}
    >
      {rows}
    </virtual-list>
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
  const { theme } = useUi()
  const go = () => sendReader({ type: 'block', index: entry.blockIndex })
  return (
    <div
      role="button"
      aria-current={active ? 'location' : undefined}
      tabIndex={0}
      onClick={go}
      onKeyDown={activateOnEnter(go)}
      style={{
        paddingTop: 5,
        paddingBottom: 5,
        paddingLeft: 10 + indent * 12,
        paddingRight: 10,
        borderRadius: 6,
        cursor: 'pointer',
        backgroundColor: active ? theme.sidebarAccent : undefined,
        hover: { backgroundColor: theme.sidebarAccent },
        userSelect: 'none',
      }}
    >
      <Label
        color={active || indent === 0 ? theme.foreground : theme.mutedForeground}
        style={{ whiteSpace: 'nowrap', textOverflow: 'ellipsis' }}
      >
        {entry.text}
      </Label>
    </div>
  )
})
