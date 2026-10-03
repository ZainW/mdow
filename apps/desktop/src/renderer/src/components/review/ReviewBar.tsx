import { useEffect, useState, type RefObject } from 'react'
import { Check, ChevronDown, ChevronUp, LoaderCircle, Sparkles } from 'lucide-react'
import type { DocumentReview } from '../../hooks/useDocumentReview'
import { scrollBehavior } from '../../lib/motion'
import { cn, isMac } from '../../lib/utils'
import { useAppStore } from '../../store/app-store'
import { Button } from '../ui/button'
import { Kbd } from '../ui/kbd'

const CHANGE_SELECTOR = '[data-change-index]'

function isEditable(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName))
  )
}

function changeElements(container: HTMLElement | null): HTMLElement[] {
  return container ? Array.from(container.querySelectorAll<HTMLElement>(CHANGE_SELECTOR)) : []
}

function focusChange(container: HTMLElement | null, index: number, onlyIfHidden = false) {
  const changes = changeElements(container)
  const target = changes[index]
  if (!target) return
  for (const change of changes) delete change.dataset.reviewActive
  // Only stepping marks a change; the first one shown on arrival needs no emphasis.
  if (!onlyIfHidden) target.dataset.reviewActive = ''
  if (onlyIfHidden) {
    const rect = target.getBoundingClientRect()
    if (rect.top >= 0 && rect.bottom <= window.innerHeight - 96) return
  }
  target.scrollIntoView({ block: 'center', behavior: scrollBehavior('travel') })
}

/**
 * Floats over the bottom of the reader while the companion suggests a change to the open
 * document: steps through each change and accepts or declines the whole suggestion.
 */
export function ReviewBar({
  review,
  changeCount,
  containerRef,
}: {
  review: DocumentReview
  changeCount: number
  containerRef: RefObject<HTMLElement | null>
}) {
  const decide = useAppStore((state) => state.reviewCompanionChange)
  const [index, setIndex] = useState(0)
  const permissionId = review.permissionId
  const deciding = review.status === 'pending' && !permissionId
  const applied = review.status === 'applied'

  // Bring the first change into view when the suggestion arrives. Callers key the bar by
  // suggestion, so a new one starts fresh at the first change.
  useEffect(() => {
    if (changeCount === 0) return undefined
    const frame = requestAnimationFrame(() => focusChange(containerRef.current, 0, true))
    return () => cancelAnimationFrame(frame)
  }, [changeCount, containerRef])

  useEffect(() => {
    if (!permissionId) return undefined
    const onKeyDown = (event: KeyboardEvent) => {
      const mod = isMac ? event.metaKey : event.ctrlKey
      if (!mod || event.altKey || event.shiftKey || isEditable(event.target)) return
      if (event.key === 'Enter') {
        event.preventDefault()
        void decide(permissionId, 'approve')
      } else if (event.key === 'Backspace') {
        event.preventDefault()
        void decide(permissionId, 'reject')
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [permissionId, decide])

  const go = (step: number) => {
    if (changeCount === 0) return
    const next = (index + step + changeCount) % changeCount
    setIndex(next)
    focusChange(containerRef.current, next)
  }

  const mod = isMac ? '⌘' : 'Ctrl'

  return (
    // Zero-height sticky rail: the bar floats over the document instead of pushing it down.
    <div className="pointer-events-none sticky bottom-0 z-20 h-0">
      <div className="absolute inset-x-0 bottom-5 flex justify-center px-4">
        <section
          aria-label="Suggested edit"
          className="review-bar pointer-events-auto flex max-w-full items-center gap-1 rounded-xl bg-popover p-1 pl-3 text-sm text-popover-foreground shadow-lg ring-1 ring-foreground/10 dark:shadow-none"
        >
          <span className="flex min-w-0 items-center gap-2 pr-1">
            {deciding ? (
              <LoaderCircle
                className="size-3.5 shrink-0 text-muted-foreground motion-safe:animate-spin"
                aria-hidden
              />
            ) : applied ? (
              <Check className="size-3.5 shrink-0 text-emerald-600" aria-hidden />
            ) : (
              <Sparkles className="size-3.5 shrink-0 text-muted-foreground" aria-hidden />
            )}
            <span className="truncate font-medium" aria-live="polite">
              {applied ? 'Edit applied' : deciding ? 'Applying edit…' : 'Suggested edit'}
            </span>
          </span>

          {changeCount > 1 && (
            <span className="flex items-center border-l border-border-subtle pl-1">
              <Button
                size="icon-xs"
                variant="ghost"
                aria-label="Previous change"
                onClick={() => go(-1)}
              >
                <ChevronUp />
              </Button>
              <span className="min-w-14 text-center text-xs text-muted-foreground tabular-nums">
                {index + 1} of {changeCount}
              </span>
              <Button size="icon-xs" variant="ghost" aria-label="Next change" onClick={() => go(1)}>
                <ChevronDown />
              </Button>
            </span>
          )}

          {permissionId && (
            <span
              className={cn(
                'flex items-center gap-1 pl-1',
                changeCount > 1 && 'border-l border-border-subtle',
              )}
            >
              <Button
                size="sm"
                variant="ghost"
                className="gap-1.5"
                title={`Decline (${mod}⌫)`}
                onClick={() => void decide(permissionId, 'reject')}
              >
                Decline
              </Button>
              <Button
                size="sm"
                className="gap-1.5"
                title={`Accept (${mod}↵)`}
                onClick={() => void decide(permissionId, 'approve')}
              >
                <Check aria-hidden />
                Accept
                <Kbd className="hidden bg-primary-foreground/15 text-primary-foreground sm:inline-flex">
                  {mod}↵
                </Kbd>
              </Button>
            </span>
          )}
        </section>
      </div>
    </div>
  )
}
