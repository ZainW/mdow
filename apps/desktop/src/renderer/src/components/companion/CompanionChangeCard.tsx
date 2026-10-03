import { Check, FilePen, FilePlus, LoaderCircle, Undo2, X } from 'lucide-react'
import type { CompanionChangeStatus, CompanionPart } from '../../../../shared/types'
import { cn } from '../../lib/utils'
import { useAppStore } from '../../store/app-store'
import { FileDiff } from '../review/FileDiff'
import { Button } from '../ui/button'

type ChangePart = Extract<CompanionPart, { kind: 'change' }>

function openDocument(path: string) {
  window.dispatchEvent(new CustomEvent('mdow:open-document-link', { detail: { path } }))
}

function StatusPill({ status, applying }: { status: CompanionChangeStatus; applying: boolean }) {
  if (applying) {
    return (
      <span className="inline-flex items-center gap-1 text-muted-foreground">
        <LoaderCircle className="size-3 motion-safe:animate-spin" aria-hidden />
        Applying
      </span>
    )
  }
  switch (status) {
    case 'pending':
      return (
        <span className="inline-flex items-center gap-1.5 text-amber-700 dark:text-amber-400">
          <span className="size-1.5 rounded-full bg-current" aria-hidden />
          Review
        </span>
      )
    case 'applied':
      return (
        <span className="inline-flex items-center gap-1 text-emerald-700 dark:text-emerald-400">
          <Check className="size-3" aria-hidden />
          Applied
        </span>
      )
    case 'rejected':
      return (
        <span className="inline-flex items-center gap-1 text-muted-foreground">
          <Undo2 className="size-3" aria-hidden />
          Declined
        </span>
      )
    default:
      return (
        <span className="inline-flex items-center gap-1 text-destructive">
          <X className="size-3" aria-hidden />
          Failed
        </span>
      )
  }
}

export function CompanionChangeCard({ part }: { part: ChangePart }) {
  const review = useAppStore((state) => state.reviewCompanionChange)
  const awaitingReview = part.status === 'pending' && Boolean(part.permissionId)
  const applying = part.status === 'pending' && !part.permissionId
  const settled = part.status === 'rejected' || part.status === 'failed'
  const additions = part.files.reduce((sum, file) => sum + file.additions, 0)
  const deletions = part.files.reduce((sum, file) => sum + file.deletions, 0)
  const title =
    part.files.length === 0
      ? 'Proposed change'
      : part.files.length === 1
        ? part.files[0].displayPath
        : `${part.files.length} documents`
  const Icon =
    part.files.every((file) => file.status === 'added') && part.files.length > 0
      ? FilePlus
      : FilePen

  return (
    <section
      aria-label={`Change to ${title}`}
      className={cn(
        'not-prose overflow-hidden rounded-lg border bg-background text-xs',
        awaitingReview ? 'border-amber-500/40 shadow-sm dark:shadow-none' : 'border-border-subtle',
        settled && 'opacity-75',
      )}
    >
      <header className="flex min-h-9 items-center gap-2 border-b border-border-subtle bg-muted/40 px-3">
        <Icon className="size-3.5 shrink-0 text-muted-foreground" aria-hidden />
        <span className="min-w-0 flex-1 truncate font-medium text-foreground" title={title}>
          {title}
        </span>
        {(additions > 0 || deletions > 0) && (
          <span className="shrink-0 font-mono text-[11px] tabular-nums">
            <span className="text-emerald-700 dark:text-emerald-400">+{additions}</span>{' '}
            <span className="text-red-700 dark:text-red-400">−{deletions}</span>
          </span>
        )}
        <span className="shrink-0 text-[11px] font-medium">
          <StatusPill status={part.status} applying={applying} />
        </span>
      </header>

      {part.files.map((file) => (
        <div key={file.path} className="border-b border-border-subtle last:border-b-0">
          {part.files.length > 1 && (
            <p className="truncate bg-muted/20 px-3 py-1 font-mono text-[11px] text-muted-foreground">
              {file.displayPath}
            </p>
          )}
          <FileDiff file={file} collapsedByDefault={settled} />
        </div>
      ))}

      {part.error && part.status === 'failed' && (
        <p className="border-t border-border-subtle px-3 py-2 text-destructive">{part.error}</p>
      )}

      {(awaitingReview || part.status === 'applied') && part.files.length > 0 && (
        <footer className="flex items-center gap-1.5 border-t border-border-subtle px-2 py-1.5">
          {part.files.length === 1 && part.files[0].status !== 'deleted' && (
            <Button
              size="xs"
              variant="ghost"
              className="text-muted-foreground"
              onClick={() => openDocument(part.files[0].path)}
            >
              {awaitingReview ? 'Review in document' : 'Open document'}
            </Button>
          )}
          {awaitingReview && part.permissionId && (
            <div className="ml-auto flex items-center gap-1.5">
              <Button
                size="xs"
                variant="outline"
                onClick={() => void review(part.permissionId!, 'reject')}
              >
                Decline
              </Button>
              <Button size="xs" onClick={() => void review(part.permissionId!, 'approve')}>
                <Check aria-hidden />
                Accept
              </Button>
            </div>
          )}
        </footer>
      )}
      {awaitingReview && part.files.length === 0 && part.permissionId && (
        <footer className="flex items-center justify-end gap-1.5 px-2 py-1.5">
          <Button
            size="xs"
            variant="outline"
            onClick={() => void review(part.permissionId!, 'reject')}
          >
            Decline
          </Button>
          <Button size="xs" onClick={() => void review(part.permissionId!, 'approve')}>
            Accept
          </Button>
        </footer>
      )}
    </section>
  )
}
