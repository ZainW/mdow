import type { ReactNode } from 'react'
import { cn } from '~/lib/utils'
import {
  ChevronDownIcon,
  ChevronRightIcon,
  FileIcon,
  FolderIcon,
  SearchIcon,
  SparkIcon,
} from '../icons'

/**
 * Small HTML illustrations of Mdow's UI for the feature grid. They are drawn
 * with site tokens so they stay crisp and follow the site theme. Decorative
 * only: each exposes a single accessible description.
 */
function Illustration({
  label,
  className,
  children,
}: {
  label: string
  className?: string
  children: ReactNode
}) {
  return (
    <div role="img" aria-label={label} className={cn('illustration', className)}>
      <div aria-hidden>{children}</div>
    </div>
  )
}

function Pane({ className, children }: { className?: string; children: ReactNode }) {
  return (
    <div
      className={cn(
        'overflow-hidden rounded-xl border border-border-subtle bg-background text-[13px] shadow-[0_1px_2px_hsl(var(--shadow-color)/0.05)]',
        className,
      )}
    >
      {children}
    </div>
  )
}

export function CompanionMock() {
  return (
    <Illustration label="The Mdow AI companion answering a question about architecture.md with clickable source citations">
      <Pane className="mx-auto max-w-md">
        <div className="flex items-center justify-between border-b border-border-subtle px-4 py-2.5">
          <span className="flex items-center gap-2 font-medium">
            <SparkIcon className="size-3.5 text-accent" />
            Companion
          </span>
          <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <span className="size-1.5 rounded-full bg-[oklch(0.68_0.15_150)]" />
            OpenCode
          </span>
        </div>
        <div className="space-y-3 p-4">
          <div className="ml-auto w-fit max-w-[85%] rounded-2xl rounded-br-md bg-muted px-3.5 py-2 leading-snug">
            How does sync resolve conflicts?{' '}
            <span className="rounded-md bg-accent/12 px-1 py-px font-mono text-[11.5px] text-accent">
              @docs/sync.md
            </span>
          </div>
          <div className="max-w-[92%] leading-relaxed text-foreground/90">
            Each device keeps a vector clock. When two edits race, the one with the later clock wins
            and the other is kept as a sibling revision
            <Cite n={1} />. Nothing is discarded silently
            <Cite n={2} />.
          </div>
          <div className="flex flex-wrap gap-1.5 pt-1">
            <SourceChip n={1} path="sync.md" line="42" />
            <SourceChip n={2} path="decisions/007.md" line="12" />
          </div>
        </div>
      </Pane>
    </Illustration>
  )
}

function Cite({ n }: { n: number }) {
  return (
    <sup className="ml-0.5 inline-flex size-4 -translate-y-px items-center justify-center rounded bg-link/12 font-mono text-[10px] font-medium text-link">
      {n}
    </sup>
  )
}

function SourceChip({ n, path, line }: { n: number; path: string; line: string }) {
  return (
    <span className="inline-flex items-center gap-1.5 rounded-md border border-border-subtle bg-card px-2 py-1 font-mono text-[11px] text-muted-foreground">
      <span className="text-link">{n}</span>
      {path}
      <span className="text-faint">:{line}</span>
    </span>
  )
}

export function SpeedMock() {
  return (
    <Illustration label="A 9 megabyte markdown file opens in about one and a half seconds">
      <div className="flex items-end gap-3">
        <span className="font-display text-[4.5rem] leading-[0.85] tracking-tight">1.5</span>
        <span className="pb-1 text-sm text-muted-foreground">
          seconds to open
          <br />a 9 MB document
        </span>
      </div>
      <div className="mt-6 space-y-2.5 text-xs">
        <SpeedBar label="v1.10" width="18%" value="1.5 s" highlight />
        <SpeedBar label="v1.9" width="90%" value="5× slower" />
      </div>
    </Illustration>
  )
}

function SpeedBar({
  label,
  width,
  value,
  highlight = false,
}: {
  label: string
  width: string
  value: string
  highlight?: boolean
}) {
  return (
    <div className="grid grid-cols-[2.75rem_1fr] items-center gap-2">
      <span className="font-mono text-muted-foreground">{label}</span>
      <div className="flex items-center gap-2">
        <div
          className={cn('h-2 rounded-full', highlight ? 'bg-accent' : 'bg-border-strong')}
          style={{ width }}
        />
        <span className="shrink-0 font-mono tabular-nums text-muted-foreground">{value}</span>
      </div>
    </div>
  )
}

export function CodeMock() {
  const k = 'text-code-keyword'
  const s = 'text-code-string'
  const f = 'text-code-function'
  const t = 'text-code-type'
  const c = 'text-code-comment'
  return (
    <Illustration label="A TypeScript code block with syntax highlighting">
      <Pane>
        <div className="flex items-center justify-between border-b border-border-subtle px-3.5 py-2 font-mono text-[11px] text-muted-foreground">
          <span>sync.ts</span>
          <span>ts</span>
        </div>
        <pre className="overflow-hidden px-3.5 py-3 font-mono text-[12px] leading-[1.7]">
          <code>
            <span className={c}>{'// Later clock wins'}</span>
            {'\n'}
            <span className={k}>function</span> <span className={f}>merge</span>
            {'(a: '}
            <span className={t}>Rev</span>
            {', b: '}
            <span className={t}>Rev</span>
            {') {'}
            {'\n  '}
            <span className={k}>if</span> {'(a.clock > b.clock)'}
            {'\n    '}
            <span className={k}>return</span> {'a'}
            {'\n  '}
            <span className={k}>return</span> {'{ ...b, tag: '}
            <span className={s}>{"'sibling'"}</span>
            {' }'}
            {'\n}'}
          </code>
        </pre>
      </Pane>
    </Illustration>
  )
}

export function DiagramMock() {
  return (
    <Illustration label="A Mermaid flowchart rendered inline: Edit, Save, Watch, Render">
      <svg viewBox="0 0 260 112" className="h-auto w-full" fill="none">
        <defs>
          <marker
            id="arrow"
            viewBox="0 0 8 8"
            refX="7"
            refY="4"
            markerWidth="7"
            markerHeight="7"
            orient="auto-start-reverse"
          >
            <path d="M0 0 8 4 0 8Z" className="fill-muted-foreground" />
          </marker>
        </defs>
        <g className="stroke-border-strong" strokeWidth="1.25">
          <path d="M70 30h40" markerEnd="url(#arrow)" />
          <path d="M150 30h40" markerEnd="url(#arrow)" />
          <path d="M220 46v26a10 10 0 0 1-10 10H160" markerEnd="url(#arrow)" />
          <path d="M100 82H53a10 10 0 0 1-10-10V48" markerEnd="url(#arrow)" strokeDasharray="3 3" />
        </g>
        <DiagramNode x={16} y={14} w={54} label="Edit" />
        <DiagramNode x={112} y={14} w={38} label="Save" />
        <DiagramNode x={192} y={14} w={58} label="Watch" />
        <DiagramNode x={102} y={66} w={58} label="Render" accent />
      </svg>
    </Illustration>
  )
}

function DiagramNode({
  x,
  y,
  w,
  label,
  accent = false,
}: {
  x: number
  y: number
  w: number
  label: string
  accent?: boolean
}) {
  return (
    <g>
      <rect
        x={x}
        y={y}
        width={w}
        height={32}
        rx={7}
        className={accent ? 'fill-accent/10 stroke-accent' : 'fill-card stroke-border-strong'}
        strokeWidth="1.25"
      />
      <text
        x={x + w / 2}
        y={y + 20}
        textAnchor="middle"
        fontSize="11.5"
        className={cn('font-sans', accent ? 'fill-accent' : 'fill-foreground')}
        fontWeight={500}
      >
        {label}
      </text>
    </g>
  )
}

export function TreeMock() {
  const rows: {
    depth: number
    name: string
    type: 'dir' | 'file'
    open?: boolean
    active?: boolean
  }[] = [
    { depth: 0, name: 'atlas', type: 'dir', open: true },
    { depth: 1, name: 'docs', type: 'dir', open: true },
    { depth: 2, name: 'architecture.md', type: 'file' },
    { depth: 2, name: 'sync.md', type: 'file', active: true },
    { depth: 2, name: 'roadmap.md', type: 'file' },
    { depth: 1, name: 'notes', type: 'dir' },
    { depth: 1, name: 'README.md', type: 'file' },
  ]
  return (
    <Illustration label="A folder tree in the Mdow sidebar with sync.md selected">
      <Pane className="py-2">
        {rows.map((row) => (
          <div
            key={row.name}
            className={cn(
              'mx-1.5 flex items-center gap-1.5 rounded-md py-[5px] pr-2 text-[12.5px]',
              row.active ? 'bg-muted font-medium text-foreground' : 'text-muted-foreground',
            )}
            style={{ paddingLeft: `${row.depth * 14 + 8}px` }}
          >
            {row.type === 'dir' ? (
              <>
                {row.open ? (
                  <ChevronDownIcon className="size-3 text-faint" />
                ) : (
                  <ChevronRightIcon className="size-3 text-faint" />
                )}
                <FolderIcon className="size-3.5" />
              </>
            ) : (
              <>
                <span className="w-3" />
                <FileIcon className={cn('size-3.5', row.active && 'text-accent')} />
              </>
            )}
            {row.name}
          </div>
        ))}
      </Pane>
    </Illustration>
  )
}

export function PaletteMock({ modKey }: { modKey: string }) {
  const items = [
    { label: 'sync.md', hint: 'docs', active: true },
    { label: 'Toggle split view', hint: 'Action' },
    { label: 'Toggle sidebar', hint: `${modKey}B` },
    { label: 'Find in document', hint: `${modKey}F` },
  ]
  return (
    <Illustration label="The Mdow command palette listing files and actions">
      <Pane className="mx-auto max-w-sm shadow-[0_12px_32px_-12px_hsl(var(--shadow-color)/0.25)]">
        <div className="flex items-center gap-2.5 border-b border-border-subtle px-3.5 py-3">
          <SearchIcon className="size-4 text-muted-foreground" />
          <span className="text-foreground">sy</span>
          <span className="-ml-2 h-4 w-px animate-pulse-dot bg-foreground" />
          <span className="kbd ml-auto">{modKey}K</span>
        </div>
        <div className="p-1.5">
          {items.map((item) => (
            <div
              key={item.label}
              className={cn(
                'flex items-center justify-between rounded-lg px-2.5 py-2',
                item.active && 'bg-muted',
              )}
            >
              <span className={item.active ? 'font-medium' : 'text-muted-foreground'}>
                {item.label}
              </span>
              <span className="font-mono text-[11px] text-faint">{item.hint}</span>
            </div>
          ))}
        </div>
      </Pane>
    </Illustration>
  )
}

export function LiveReloadMock() {
  return (
    <Illustration label="Saving a file in a text editor updates the rendered document in Mdow">
      <div className="grid grid-cols-2 gap-3">
        <Pane>
          <div className="border-b border-border-subtle px-3 py-2 font-mono text-[11px] text-muted-foreground">
            your editor
          </div>
          <div className="space-y-1 px-3 py-3 font-mono text-[11.5px] leading-relaxed">
            <div className="text-muted-foreground">## Launch plan</div>
            <div className="text-muted-foreground">- Ship **v2** beta</div>
            <div className="-mx-1 rounded bg-accent/10 px-1 text-foreground">
              - Invite 50 testers
              <span className="ml-px inline-block h-3.5 w-px translate-y-0.5 bg-foreground" />
            </div>
          </div>
        </Pane>
        <Pane>
          <div className="flex items-center justify-between border-b border-border-subtle px-3 py-2 text-[11px] text-muted-foreground">
            <span className="font-mono">mdow</span>
            <span className="flex items-center gap-1.5">
              <span className="size-1.5 animate-pulse-dot rounded-full bg-accent" />
              live
            </span>
          </div>
          <div className="px-3 py-3 text-[12px] leading-relaxed">
            <div className="font-semibold">Launch plan</div>
            <ul className="mt-1 list-disc space-y-0.5 pl-4 text-foreground/85 marker:text-faint">
              <li>
                Ship <strong>v2</strong> beta
              </li>
              <li className="-ml-1 rounded bg-accent/10 pl-1">Invite 50 testers</li>
            </ul>
          </div>
        </Pane>
      </div>
    </Illustration>
  )
}
