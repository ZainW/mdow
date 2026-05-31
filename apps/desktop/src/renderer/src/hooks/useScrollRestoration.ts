import { useEffect, useLayoutEffect, useRef, type RefObject } from 'react'

export function useScrollRestoration({
  scrollRef,
  tabId,
  scrollPosition,
  updateTabScroll,
}: {
  scrollRef: RefObject<HTMLDivElement | null>
  tabId: string
  scrollPosition: number
  updateTabScroll: (tabId: string, scrollPosition: number) => void
}): void {
  const prevTabIdRef = useRef(tabId)

  useEffect(() => {
    const el = scrollRef.current
    if (!el) return undefined
    let timer: ReturnType<typeof setTimeout>
    const handler = () => {
      clearTimeout(timer)
      timer = setTimeout(() => {
        updateTabScroll(tabId, el.scrollTop)
      }, 150)
    }
    el.addEventListener('scroll', handler, { passive: true })
    return () => {
      clearTimeout(timer)
      el.removeEventListener('scroll', handler)
    }
  }, [scrollRef, tabId, updateTabScroll])

  useLayoutEffect(() => {
    if (prevTabIdRef.current !== tabId) {
      const el = scrollRef.current
      if (el) {
        el.scrollTo(0, scrollPosition)
      }
      prevTabIdRef.current = tabId
    }
  }, [scrollRef, tabId, scrollPosition])
}
