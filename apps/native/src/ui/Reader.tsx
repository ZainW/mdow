import { memo, useEffect, useMemo, useRef, useState } from 'react'
import {
  findRanges,
  useGpuixRequired,
  useTextSearch,
  type EventPayload,
  type PublicInstance,
} from '@gpuix/react'
import type { AlertKind, Block, ParsedDocument } from '../lib/markdown'
import { copyToClipboard } from '../lib/platform'
import { setOverlay, useApp } from '../store'
import { METRICS, useUi } from './context'
import { Icon, IconButton, Label } from './primitives'
import { onReaderCommand } from './reader-bus'

const WINDOW = 120
const ESTIMATED_ROW = 64
const LINE_STEP = 48
/** Page keys move 90% of the viewport so a line of context stays on screen. */
const PAGE_FRACTION = 0.9

/** Per-tab scroll anchors: [row index, offset in row]. Survives tab switches. */
const scrollMemory = new Map<string, [number, number]>()

export interface ReaderProps {
  path: string
  document: ParsedDocument
  columnWidth: number
  /** Left edge of the reading column inside the reader pane. */
  inset: number
  onLink: (href: string) => void
}

export function Reader({ path, document, columnWidth, inset, onLink }: ReaderProps) {
  const renderer = useGpuixRequired()
  const { theme } = useUi()
  const listRef = useRef<PublicInstance | null>(null)
  const blocks = document.blocks
  const [start, setStart] = useState(() => {
    const saved = scrollMemory.get(path)
    return saved ? Math.max(0, saved[0] - WINDOW / 4) : 0
  })
  const windowStart = Math.min(start, Math.max(0, blocks.length - 1))
  const end = Math.min(blocks.length, windowStart + WINDOW)

  const findOpen = useApp((state) => state.overlay === 'find')
  const [query, setQuery] = useState('')
  const activeQuery = findOpen ? query : ''
  const perRow = useMemo(
    () =>
      activeQuery
        ? blocks.map((block) => findRanges({ text: block.text, query: activeQuery }).length)
        : null,
    [blocks, activeQuery],
  )
  const total = perRow ? perRow.reduce((sum, count) => sum + count, 0) : 0
  const indexOffset = perRow
    ? perRow.slice(0, windowStart).reduce((sum, count) => sum + count, 0)
    : 0
  const search = useTextSearch({
    query: activeQuery,
    color: theme.findMatch,
    activeColor: theme.findActive,
    matches: { total, indexOffset },
  })

  // Restore this tab's scroll anchor once the list has laid out.
  useEffect(() => {
    const saved = scrollMemory.get(path)
    const id = listRef.current?.id
    if (id === undefined) return
    if (saved) renderer.scrollToItem?.(id, saved[0], saved[1])
    return () => {
      const anchor = renderer.getListScrollTop?.(id)
      if (anchor) scrollMemory.set(path, [anchor[0] ?? 0, anchor[1] ?? 0])
    }
  }, [path, renderer])

  const scrollToRow = (index: number, offset = 0) => {
    const id = listRef.current?.id
    if (id === undefined) return
    setStart((current) =>
      index >= current && index < current + WINDOW ? current : Math.max(0, index - WINDOW / 4),
    )
    renderer.scrollToItem?.(id, Math.max(0, Math.min(index, blocks.length)), offset)
  }

  const scrollBy = (pixels: number) => {
    const id = listRef.current?.id
    const anchor = id !== undefined ? renderer.getListScrollTop?.(id) : null
    if (!anchor || id === undefined) return
    const [index = 0, offset = 0] = anchor
    const next = offset + pixels
    renderer.scrollToItem?.(id, index, index === 0 ? Math.max(0, next) : next)
  }

  const rowOfMatch = (match: number) => {
    if (!perRow) return 0
    let seen = 0
    for (let row = 0; row < perRow.length; row++) {
      seen += perRow[row]!
      if (match < seen) return row
    }
    return perRow.length - 1
  }

  // Latest closures for the long-lived command listener below.
  const searchRef = useRef(search)
  const handlers = useRef({ scrollToRow, scrollBy, rowOfMatch })
  useEffect(() => {
    searchRef.current = search
    handlers.current = { scrollToRow, scrollBy, rowOfMatch }
  })

  useEffect(
    () =>
      onReaderCommand((command) => {
        const { scrollToRow: toRow, scrollBy: by } = handlers.current
        const id = listRef.current?.id
        const viewport = id !== undefined ? (renderer.getListScrollTop?.(id)?.[2] ?? 600) : 600
        switch (command.type) {
          case 'scroll':
            toRow(command.to === 'top' ? 0 : blocks.length)
            break
          case 'page':
            by(command.direction * viewport * PAGE_FRACTION)
            break
          case 'line':
            by(command.direction * LINE_STEP)
            break
          case 'block':
            toRow(command.index)
            break
          case 'slug': {
            const index = document.slugs.get(command.slug)
            if (index !== undefined) toRow(index)
            break
          }
          case 'find':
            if (command.direction > 0) searchRef.current.next()
            else searchRef.current.previous()
            break
        }
      }),
    [blocks.length, document.slugs, renderer],
  )

  // Keep the active match on screen as the find cursor moves.
  useEffect(() => {
    if (!activeQuery || total === 0) return
    handlers.current.scrollToRow(handlers.current.rowOfMatch(search.active))
  }, [activeQuery, search.active, total])

  const rows = []
  for (let index = windowStart; index < end; index++) {
    const block = blocks[index]!
    rows.push(
      <Row
        key={block.id}
        block={block}
        first={index === 0}
        last={index === blocks.length - 1}
        columnWidth={columnWidth}
        inset={inset}
        onLink={onLink}
      />,
    )
  }

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        flexGrow: 1,
        minHeight: 0,
        position: 'relative',
      }}
    >
      <div
        {...search.props}
        testId="reader-pane"
        style={{ display: 'flex', flexDirection: 'column', flexGrow: 1, minHeight: 0 }}
      >
        {blocks.length === 0 ? (
          <div
            style={{ padding: METRICS.readerTopPadding, display: 'flex', justifyContent: 'center' }}
          >
            <Label color={theme.mutedForeground}>This document is empty.</Label>
          </div>
        ) : (
          <virtual-list
            ref={listRef}
            testId="reader"
            itemCount={blocks.length}
            windowStart={windowStart}
            estimatedItemHeight={ESTIMATED_ROW}
            overdraw={800}
            onVisibleRange={(event) => {
              const first = Math.floor(event.startIndex ?? 0)
              const last = Math.ceil(event.endIndex ?? first)
              if (first < windowStart + WINDOW / 8 || last > end - WINDOW / 8) {
                const next = Math.max(0, first - WINDOW / 4)
                if (next !== windowStart) setStart(next)
              }
            }}
            style={{ flexGrow: 1, minHeight: 0 }}
          >
            {rows}
          </virtual-list>
        )}
      </div>
      {findOpen ? (
        <FindBar
          query={query}
          onQuery={setQuery}
          total={total}
          active={search.active}
          onNext={() => search.next()}
          onPrevious={() => search.previous()}
        />
      ) : null}
    </div>
  )
}

const Row = memo(function Row({
  block,
  first,
  last,
  columnWidth,
  inset,
  onLink,
}: {
  block: Block
  first: boolean
  last: boolean
  columnWidth: number
  inset: number
  onLink: (href: string) => void
}) {
  const { reader } = useUi()
  const gap = reader.fontSize
  return (
    <div
      style={{
        display: 'flex',
        paddingLeft: inset,
        paddingTop: first ? METRICS.readerTopPadding : 0,
        paddingBottom: last ? METRICS.readerBottomPadding : gap,
      }}
    >
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          // Row widths inside a virtual list aren't definite, so percentages and centering
          // don't resolve reliably. Place the column with explicit pixels instead.
          width: columnWidth,
          minWidth: 0,
        }}
      >
        <BlockView block={block} columnWidth={columnWidth} onLink={onLink} />
      </div>
    </div>
  )
})

function BlockView({
  block,
  columnWidth,
  onLink,
}: {
  block: Block
  columnWidth: number
  onLink: (href: string) => void
}) {
  const { reader, theme } = useUi()
  const link = (event: EventPayload) => {
    if (event.value) onLink(event.value)
  }
  switch (block.kind) {
    case 'markdown':
    case 'heading':
      return <markdown source={block.source} theme={reader.native} onLinkClick={link} />
    case 'code':
      return <CodeBlock language={block.language} code={block.code} />
    case 'alert':
      return <Alert kind={block.alert} source={block.source} onLink={link} />
    case 'image': {
      if (!block.src) {
        return (
          <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
            <Icon name="alert-circle" size={14} />
            <Label color={theme.mutedForeground} size={reader.fontSize * 0.875}>
              {block.alt || 'Image unavailable'}
            </Label>
          </div>
        )
      }
      const width = Math.min(block.width, columnWidth)
      const height = Math.round((block.height / block.width) * width)
      return (
        <img
          src={block.src}
          alt={block.alt}
          objectFit="contain"
          style={{ width, height, borderRadius: 6, alignSelf: 'flex-start' }}
        />
      )
    }
    case 'rule':
      return (
        <div style={{ paddingTop: reader.fontSize * 0.5, paddingBottom: reader.fontSize * 0.5 }}>
          <div style={{ height: 1, backgroundColor: theme.border }} />
        </div>
      )
    case 'footnotes':
      return (
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 6,
            borderTopWidth: 1,
            borderColor: theme.border,
            paddingTop: reader.fontSize,
          }}
        >
          {block.items.map((note) => (
            <div key={note.label} style={{ display: 'flex', gap: 8 }}>
              <Label
                color={theme.mutedForeground}
                size={reader.fontSize * 0.875}
                style={{ fontFamily: reader.contentFont }}
              >
                {note.marker}
              </Label>
              <div style={{ flexGrow: 1, minWidth: 0 }}>
                <markdown
                  source={note.source}
                  theme={{
                    ...reader.native,
                    text: theme.mutedForeground,
                    metrics: {
                      ...reader.native.metrics,
                      mdTextSize: reader.fontSize * 0.875,
                      mdLineHeight: Math.round(reader.fontSize * 0.875 * 1.6),
                    },
                  }}
                  onLinkClick={link}
                />
              </div>
            </div>
          ))}
        </div>
      )
  }
}

const LANGUAGE_ALIASES: Record<string, string> = {
  rs: 'rust',
  rust: 'rust',
  ts: 'typescript',
  typescript: 'typescript',
  mts: 'typescript',
  cts: 'typescript',
  tsx: 'tsx',
  js: 'javascript',
  javascript: 'javascript',
  mjs: 'javascript',
  cjs: 'javascript',
  jsx: 'jsx',
  py: 'python',
  python: 'python',
  go: 'go',
  golang: 'go',
  json: 'json',
  jsonc: 'json',
  json5: 'json',
  sh: 'bash',
  bash: 'bash',
  zsh: 'bash',
  shell: 'bash',
  console: 'bash',
  toml: 'toml',
  yml: 'yaml',
  yaml: 'yaml',
  md: 'markdown',
  markdown: 'markdown',
  mdx: 'markdown',
  html: 'html',
  htm: 'html',
  xml: 'html',
  svg: 'html',
  css: 'css',
  scss: 'css',
  c: 'c',
  h: 'c',
  'c++': 'c',
  cpp: 'c',
}

export function nativeLanguage(language: string) {
  return LANGUAGE_ALIASES[language.toLowerCase()]
}

function CodeBlock({ language, code }: { language: string; code: string }) {
  const { reader, theme, scale } = useUi()
  const [copied, setCopied] = useState(false)
  useEffect(() => {
    if (!copied) return
    const timer = setTimeout(() => setCopied(false), 2000)
    return () => clearTimeout(timer)
  }, [copied])
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        borderRadius: METRICS.radius,
        borderWidth: 1,
        borderColor: theme.borderSubtle,
        backgroundColor: theme.surfaceWell,
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          paddingLeft: 14,
          paddingRight: 6,
          paddingTop: 4,
          userSelect: 'none',
        }}
      >
        <Label mono size={scale.controlXsFont + 1} color={theme.mutedForeground}>
          {language === 'mermaid' ? 'mermaid diagram' : language}
        </Label>
        <IconButton
          icon={copied ? 'check' : 'copy'}
          label={copied ? 'Copied' : 'Copy code'}
          size={scale.buttonXsHeight + 4}
          onClick={() => {
            void copyToClipboard(code).then((ok) => setCopied(ok))
          }}
        />
      </div>
      <code
        code={code}
        language={nativeLanguage(language)}
        theme={reader.native}
        style={{
          paddingLeft: 14,
          paddingRight: 14,
          paddingBottom: 12,
          paddingTop: 2,
          minWidth: 0,
          fontFamily: reader.codeFont,
          fontSize: reader.native.metrics?.codeTextSize,
          lineHeight: reader.native.metrics?.codeLineHeight,
          color: theme.foreground,
        }}
      />
    </div>
  )
}

const ALERT_TITLES: Record<AlertKind, string> = {
  note: 'Note',
  tip: 'Tip',
  important: 'Important',
  warning: 'Warning',
  caution: 'Caution',
}

function Alert({
  kind,
  source,
  onLink,
}: {
  kind: AlertKind
  source: string
  onLink: (event: EventPayload) => void
}) {
  const { reader, theme } = useUi()
  const color = theme.alerts[kind]
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: 6,
        borderLeftWidth: 3,
        borderColor: color,
        paddingLeft: 16,
        paddingTop: 2,
        paddingBottom: 2,
      }}
    >
      <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
        <Icon name="alert-circle" size={16} color={color} />
        <Label color={color} weight={600} size={reader.fontSize * 0.9375}>
          {ALERT_TITLES[kind]}
        </Label>
      </div>
      {source ? <markdown source={source} theme={reader.native} onLinkClick={onLink} /> : null}
    </div>
  )
}

function FindBar({
  query,
  onQuery,
  total,
  active,
  onNext,
  onPrevious,
}: {
  query: string
  onQuery: (query: string) => void
  total: number
  active: number
  onNext: () => void
  onPrevious: () => void
}) {
  const { theme, scale } = useUi()
  const status = !query ? '' : total === 0 ? 'No results' : `${active + 1} of ${total}`
  return (
    <div
      style={{
        position: 'absolute',
        top: 10,
        right: 16,
        display: 'flex',
        alignItems: 'center',
        gap: 4,
        height: scale.buttonHeight + 12,
        paddingLeft: 10,
        paddingRight: 4,
        borderRadius: 10,
        borderWidth: 1,
        borderColor: theme.border,
        backgroundColor: theme.surfaceRaised,
        pointerEvents: 'auto',
        boxShadow: { offsetX: 0, offsetY: 6, blurRadius: 18, spreadRadius: 0, color: '#00000026' },
      }}
    >
      <Icon name="search" size={14} />
      <input
        testId="find-input"
        autoFocus
        value={query}
        placeholder="Find in document"
        theme={{ caret: theme.primary }}
        onChange={(event) => onQuery(event.value ?? '')}
        onSubmit={onNext}
        onKeyDown={(event) => {
          if (event.key === 'escape') setOverlay(null)
          else if (event.key === 'enter' && event.modifiers?.shift) onPrevious()
        }}
        style={{
          width: 200,
          height: scale.buttonHeight,
          paddingLeft: 4,
          fontSize: scale.controlFont + 1,
          color: theme.foreground,
          fontFamily: 'Inter Variable',
        }}
      />
      <Label size={scale.controlFont} color={theme.mutedForeground} style={{ minWidth: 64 }}>
        {status}
      </Label>
      <IconButton icon="chevron-up" label="Previous match" onClick={onPrevious} />
      <IconButton icon="chevron-down" label="Next match" onClick={onNext} />
      <IconButton icon="x" label="Close find" onClick={() => setOverlay(null)} />
    </div>
  )
}
