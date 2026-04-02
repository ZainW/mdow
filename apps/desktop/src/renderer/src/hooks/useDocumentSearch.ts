import { useCallback, useEffect, useMemo, useRef, useState } from 'react'

/**
 * Highlights search matches in an HTML string by parsing it with DOMParser,
 * walking text nodes, and wrapping matches in <mark> elements.
 * Returns the modified HTML and the match count.
 *
 * This approach works WITH React (producing a new HTML string) rather than
 * fighting it (mutating the live DOM that React owns).
 *
 * Security note: The input HTML is from local markdown files rendered by md4x
 * (a trusted WASM renderer) — not from untrusted user input or external sources.
 */
function highlightHtml(html: string, query: string): { html: string; count: number } {
  if (!query || !html) return { html, count: 0 }

  const doc = new DOMParser().parseFromString(html, 'text/html')
  const walker = document.createTreeWalker(doc.body, NodeFilter.SHOW_TEXT)
  const textNodes: Text[] = []
  while (walker.nextNode()) {
    if (walker.currentNode instanceof Text) textNodes.push(walker.currentNode)
  }

  const lowerQuery = query.toLowerCase()
  let matchIndex = 0

  for (const node of textNodes) {
    const text = node.textContent || ''
    const lowerText = text.toLowerCase()
    let pos = lowerText.indexOf(lowerQuery)
    if (pos === -1) continue

    const fragment = doc.createDocumentFragment()
    let lastEnd = 0
    while (pos !== -1) {
      if (pos > lastEnd) {
        fragment.appendChild(doc.createTextNode(text.slice(lastEnd, pos)))
      }
      const mark = doc.createElement('mark')
      mark.className = 'search-highlight'
      mark.setAttribute('data-match-index', String(matchIndex++))
      mark.textContent = text.slice(pos, pos + query.length)
      fragment.appendChild(mark)
      lastEnd = pos + query.length
      pos = lowerText.indexOf(lowerQuery, lastEnd)
    }
    if (lastEnd < text.length) {
      fragment.appendChild(doc.createTextNode(text.slice(lastEnd)))
    }
    node.parentNode!.replaceChild(fragment, node)
  }

  return { html: doc.body.innerHTML, count: matchIndex }
}

export function useDocumentSearch(
  containerRef: React.RefObject<HTMLElement | null>,
  query: string,
  html: string,
) {
  const [currentIndex, setCurrentIndex] = useState(-1)
  const activeMarkRef = useRef<Element | null>(null)

  // Memoize: only recompute when html or query actually change,
  // not on every re-render (e.g. scroll-triggered store updates)
  const { html: highlightedHtml, count: matchCount } = useMemo(
    () => highlightHtml(html, query),
    [html, query],
  )

  // Reset current index when query or html changes
  const prevQueryRef = useRef(query)
  const prevHtmlRef = useRef(html)
  if (query !== prevQueryRef.current || html !== prevHtmlRef.current) {
    prevQueryRef.current = query
    prevHtmlRef.current = html
    setCurrentIndex(matchCount > 0 ? 0 : -1)
  }

  // Scroll to and style the active match after React renders the highlighted HTML
  // Only depends on currentIndex — not highlightedHtml — so scroll-triggered
  // re-renders don't strip and re-add the active class
  useEffect(() => {
    const container = containerRef.current
    if (!container || currentIndex < 0) return

    if (activeMarkRef.current) {
      activeMarkRef.current.classList.remove('search-highlight-active')
    }

    const mark = container.querySelector(`mark[data-match-index="${currentIndex}"]`)
    if (mark) {
      mark.classList.add('search-highlight-active')
      mark.scrollIntoView({ block: 'center', behavior: 'smooth' })
      activeMarkRef.current = mark
    }
  }, [currentIndex, containerRef])

  const next = useCallback(() => {
    if (matchCount > 0) {
      setCurrentIndex((i) => (i + 1) % matchCount)
    }
  }, [matchCount])

  const prev = useCallback(() => {
    if (matchCount > 0) {
      setCurrentIndex((i) => (i - 1 + matchCount) % matchCount)
    }
  }, [matchCount])

  const clear = useCallback(() => {
    setCurrentIndex(-1)
    activeMarkRef.current = null
  }, [])

  return { highlightedHtml, matchCount, currentIndex, next, prev, clear }
}
