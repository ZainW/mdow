import { useEffect, type RefObject } from 'react'
import type { RenderResult } from '../lib/markdown'
import { useAppStore } from '../store/app-store'

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
    const headingEls = container.querySelectorAll<HTMLElement>(
      'h1[id], h2[id], h3[id], h4[id], h5[id], h6[id]',
    )
    if (headingEls.length === 0) return undefined

    const observer = new IntersectionObserver(
      (entries) => {
        const visible = entries
          .filter((e) => e.isIntersecting)
          .toSorted((a, b) => a.boundingClientRect.top - b.boundingClientRect.top)
        if (visible[0]) {
          useAppStore.getState().setActiveHeadingId(visible[0].target.id)
        }
      },
      { root, rootMargin: '0px 0px -75% 0px', threshold: 0 },
    )
    for (const el of headingEls) observer.observe(el)
    return () => observer.disconnect()
  }, [scrollRef, contentRef, renderResult])
}
