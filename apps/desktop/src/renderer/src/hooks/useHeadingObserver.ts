import { useEffect, type RefObject } from 'react'
import type { RenderResult } from '../lib/markdown'
import { useAppStore } from '../store/app-store'

// A heading becomes active once its top crosses this fraction of the scroller's height.
const ACTIVE_LINE = 0.25

/** Index of the last heading whose top is at or above `line`, or -1 if none is. */
function findActiveIndex(headings: readonly HTMLElement[], line: number): number {
  let low = 0
  let high = headings.length - 1
  let found = -1
  while (low <= high) {
    const mid = (low + high) >> 1
    if (headings[mid].getBoundingClientRect().top <= line) {
      found = mid
      low = mid + 1
    } else {
      high = mid - 1
    }
  }
  return found
}

/**
 * Scroll-spy for the outline. Headings are in document order, so a binary search over their
 * positions finds the active one in O(log n) per frame; an IntersectionObserver would track
 * every heading on every frame, which adds up in documents with thousands of them.
 */
export function useHeadingObserver({
  scrollRef,
  contentRef,
  renderResult,
}: {
  scrollRef: RefObject<HTMLDivElement | null>
  contentRef: RefObject<HTMLDivElement | null>
  renderResult: RenderResult | null
}): void {
  useEffect(() => {
    const root = scrollRef.current
    const container = contentRef.current
    if (!root || !container) return undefined
    const headings = Array.from(
      container.querySelectorAll<HTMLElement>('h1[id], h2[id], h3[id], h4[id], h5[id], h6[id]'),
    )
    if (headings.length === 0) return undefined

    let frame = 0
    const update = () => {
      frame = 0
      const rect = root.getBoundingClientRect()
      const index = findActiveIndex(headings, rect.top + rect.height * ACTIVE_LINE)
      const id = headings[Math.max(index, 0)].id
      if (useAppStore.getState().activeHeadingId !== id) {
        useAppStore.getState().setActiveHeadingId(id)
      }
    }
    const schedule = () => {
      frame ||= requestAnimationFrame(update)
    }

    root.addEventListener('scroll', schedule, { passive: true })
    schedule()
    return () => {
      root.removeEventListener('scroll', schedule)
      cancelAnimationFrame(frame)
    }
  }, [scrollRef, contentRef, renderResult])
}
