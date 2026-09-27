import { useEffect, useLayoutEffect, useRef, type RefObject } from 'react'
import { captureScrollAnchor, restoreScrollPosition, type ScrollAnchor } from '../lib/scroll-anchor'

const SAVE_DEBOUNCE_MS = 150

/**
 * Remembers where the reader is in each tab and puts them back there when the tab's content is
 * rendered again: after switching tabs, and after a live reload of the same file.
 */
export function useScrollRestoration({
  scrollRef,
  contentRef,
  tabId,
  scrollPosition,
  scrollAnchor,
  renderVersion,
  updateTabScroll,
}: {
  scrollRef: RefObject<HTMLDivElement | null>
  contentRef: RefObject<HTMLDivElement | null>
  tabId: string
  scrollPosition: number
  scrollAnchor: ScrollAnchor | null | undefined
  renderVersion: number
  updateTabScroll: (tabId: string, scrollPosition: number, anchor: ScrollAnchor | null) => void
}): void {
  const savedRef = useRef({ scrollPosition, scrollAnchor })
  savedRef.current = { scrollPosition, scrollAnchor }
  const restoringRef = useRef<() => void>(() => {})

  useEffect(() => {
    const scroller = scrollRef.current
    const container = contentRef.current
    if (!scroller || !container) return undefined

    let timer: ReturnType<typeof setTimeout> | undefined
    const save = () => {
      timer = undefined
      updateTabScroll(tabId, scroller.scrollTop, captureScrollAnchor(scroller, container))
    }
    const handler = () => {
      clearTimeout(timer)
      timer = setTimeout(save, SAVE_DEBOUNCE_MS)
    }
    scroller.addEventListener('scroll', handler, { passive: true })
    return () => {
      scroller.removeEventListener('scroll', handler)
      // Switching tabs commits once more with the old document still in the DOM, so a pending
      // save can still read the right position.
      if (timer !== undefined) {
        clearTimeout(timer)
        save()
      }
    }
  }, [scrollRef, contentRef, tabId, updateTabScroll])

  useLayoutEffect(() => {
    const scroller = scrollRef.current
    const container = contentRef.current
    if (!scroller || !container || renderVersion === 0) return undefined
    const { scrollAnchor: anchor, scrollPosition: top } = savedRef.current
    restoringRef.current()
    restoringRef.current = restoreScrollPosition(scroller, container, anchor, top)
    return undefined
  }, [scrollRef, contentRef, renderVersion])

  useEffect(() => () => restoringRef.current(), [])
}
