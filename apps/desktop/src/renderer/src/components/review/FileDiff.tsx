import { useMemo, useState } from 'react'
import { ChevronDown } from 'lucide-react'
import type { CompanionFileChange } from '../../../../shared/types'
import { parseUnifiedDiff, type DiffLine } from '../../lib/unified-diff'
import { cn } from '../../lib/utils'

type DiffSize = 'compact' | 'reader'

/** Long diffs start collapsed past this many lines so the conversation stays readable. */
const PREVIEW_LINES = 14

function DiffRow({ line, size }: { line: DiffLine; size: DiffSize }) {
  const number = line.kind === 'remove' ? line.oldLine : line.newLine
  return (
    <div
      className={cn(
        size === 'compact'
          ? 'grid grid-cols-[2.25rem_1rem_1fr]'
          : 'grid grid-cols-[3rem_1.25rem_1fr]',
        line.kind === 'add' && 'bg-emerald-500/10 text-emerald-950 dark:text-emerald-100',
        line.kind === 'remove' && 'bg-red-500/10 text-red-950 dark:text-red-100',
      )}
    >
      <span className="pr-1.5 text-right text-muted-foreground-subtle tabular-nums select-none">
        {number}
      </span>
      <span
        className={cn(
          'text-center select-none',
          line.kind === 'add' && 'text-emerald-700 dark:text-emerald-400',
          line.kind === 'remove' && 'text-red-700 dark:text-red-400',
        )}
        aria-hidden
      >
        {line.kind === 'add' ? '+' : line.kind === 'remove' ? '−' : ''}
      </span>
      <span className="min-w-0 pr-2 break-words whitespace-pre-wrap">
        <span className="sr-only">
          {line.kind === 'add' ? 'Added: ' : line.kind === 'remove' ? 'Removed: ' : ''}
        </span>
        {line.text || ' '}
      </span>
    </div>
  )
}

export function FileDiff({
  file,
  collapsedByDefault = false,
  size = 'compact',
  previewLines = PREVIEW_LINES,
}: {
  file: CompanionFileChange
  collapsedByDefault?: boolean
  size?: DiffSize
  /** Lines shown before "Show all"; Infinity always shows the whole diff. */
  previewLines?: number
}) {
  const hunks = useMemo(() => parseUnifiedDiff(file.patch), [file.patch])
  const totalLines = hunks.reduce((sum, hunk) => sum + hunk.lines.length, 0)
  const [expanded, setExpanded] = useState(!collapsedByDefault)
  const showAll = expanded || totalLines <= previewLines
  const visibleHunks = useMemo(() => {
    if (showAll) return hunks
    const visible: typeof hunks = []
    let remaining = previewLines
    for (const hunk of hunks) {
      if (remaining <= 0) break
      visible.push({ ...hunk, lines: hunk.lines.slice(0, remaining) })
      remaining -= hunk.lines.length
    }
    return visible
  }, [hunks, showAll, previewLines])

  if (hunks.length === 0) {
    return (
      <p className="px-3 py-2 text-xs text-muted-foreground">
        {file.status === 'deleted' ? 'Deletes this document.' : 'No preview available.'}
      </p>
    )
  }

  return (
    <div
      className={cn(
        'font-mono',
        size === 'compact' ? 'text-[11px] leading-[1.15rem]' : 'text-[13px] leading-6',
      )}
    >
      {visibleHunks.map((hunk, hunkIndex) => (
        <div
          // Hunks never reorder within a patch.
          // oxlint-disable-next-line react/no-array-index-key
          key={hunkIndex}
          data-change-index={hunkIndex}
          className={cn(hunkIndex > 0 && 'border-t border-dashed border-border-subtle')}
        >
          {hunk.lines.map((line, lineIndex) => (
            // oxlint-disable-next-line react/no-array-index-key
            <DiffRow key={lineIndex} line={line} size={size} />
          ))}
        </div>
      ))}
      {totalLines > previewLines && (
        <button
          type="button"
          className="flex w-full items-center justify-center gap-1 border-t border-border-subtle py-1 font-sans text-[11px] text-muted-foreground hover:bg-muted/50 hover:text-foreground"
          aria-expanded={showAll}
          onClick={() => setExpanded((value) => !value)}
        >
          <ChevronDown
            className={cn('size-3 transition-transform', showAll && 'rotate-180')}
            aria-hidden
          />
          {showAll ? 'Show less' : `Show all ${totalLines} lines`}
        </button>
      )}
    </div>
  )
}
