import { FileText, FileX } from 'lucide-react'
import { canOpenDrop, describeDrop, type DropSummary } from '../lib/drop-summary'
import { cn } from '../lib/utils'

/**
 * Full-window drop target shown while files are dragged over the window.
 * Purely visual (pointer-events: none); the window root handles the drop.
 * It appears and disappears instantly: drag feedback has to track the pointer.
 */
export function DropOverlay({ summary }: { summary: DropSummary | null }) {
  if (!summary) return null
  const openable = canOpenDrop(summary)
  const Icon = openable ? FileText : FileX

  return (
    <div className="pointer-events-none fixed inset-0 z-(--z-overlay) bg-background/95 p-3">
      <DropTarget openable={openable} summaryText={describeDrop(summary)} Icon={Icon} />
    </div>
  )
}

function DropTarget({
  openable,
  summaryText,
  Icon,
}: {
  openable: boolean
  summaryText: string
  Icon: typeof FileText
}) {
  return (
    <div
      data-testid="drop-overlay"
      data-openable={openable}
      className={cn(
        'flex size-full flex-col items-center justify-center gap-2.5 rounded-xl border-[1.5px] border-dashed',
        openable
          ? 'border-primary/70 bg-primary/[0.07] text-primary'
          : 'border-muted-foreground/50 bg-muted/60 text-muted-foreground',
      )}
    >
      <div
        className={cn(
          'flex size-11 items-center justify-center rounded-[10px]',
          openable ? 'bg-primary/15' : 'bg-muted',
        )}
      >
        <Icon className="size-5" aria-hidden />
      </div>
      <p className="text-sm font-semibold text-foreground">
        {openable ? 'Drop to open' : 'Can’t open these'}
      </p>
      <output className="block text-xs text-muted-foreground">{summaryText}</output>
    </div>
  )
}
