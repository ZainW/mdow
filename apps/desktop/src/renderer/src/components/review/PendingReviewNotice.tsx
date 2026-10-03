import { ArrowRight, Sparkles } from 'lucide-react'
import { usePendingReviewPaths } from '../../hooks/usePendingReviews'
import { basename } from '../../lib/path-utils'
import { useAppStore } from '../../store/app-store'
import { Button } from '../ui/button'

/** Paths of the documents currently on screen, in one pane or both. */
function useVisiblePaths(): ReadonlySet<string> {
  const joined = useAppStore((state) => {
    const ids = state.splitView
      ? [state.primaryPaneTabId, state.secondaryPaneTabId]
      : [state.activeTabId]
    return ids
      .map((id) => state.tabs.find((tab) => tab.id === id)?.path)
      .filter(Boolean)
      .join('\n')
  })
  return new Set(joined ? joined.split('\n') : [])
}

/**
 * Points to a companion edit that is waiting for review in a document that is not on screen.
 * Reviewing opens that document, where the edit is shown in place with its review bar.
 */
export function PendingReviewNotice() {
  const pending = usePendingReviewPaths()
  const visible = useVisiblePaths()
  const elsewhere = [...pending].filter((path) => !visible.has(path))
  if (elsewhere.length === 0 || elsewhere.length < pending.size) return null

  const [first] = elsewhere
  const label =
    elsewhere.length === 1 ? basename(first) : `${basename(first)} and ${elsewhere.length - 1} more`

  return (
    <div className="pointer-events-none absolute inset-x-0 bottom-5 z-20 flex justify-center px-4">
      <section
        aria-label="Suggested edit in another document"
        className="review-bar pointer-events-auto flex max-w-full items-center gap-1 rounded-xl bg-popover p-1 pl-3 text-sm text-popover-foreground shadow-lg ring-1 ring-foreground/10 dark:shadow-none"
      >
        <Sparkles className="size-3.5 shrink-0 text-muted-foreground" aria-hidden />
        <span className="min-w-0 truncate pr-1 pl-1">
          <span className="font-medium">Suggested edit</span>
          <span className="text-muted-foreground"> to {label}</span>
        </span>
        <Button
          size="sm"
          className="gap-1.5"
          onClick={() =>
            window.dispatchEvent(
              new CustomEvent('mdow:open-document-link', { detail: { path: first } }),
            )
          }
        >
          Review
          <ArrowRight aria-hidden />
        </Button>
      </section>
    </div>
  )
}
