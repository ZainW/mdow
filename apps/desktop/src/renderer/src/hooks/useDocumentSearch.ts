import { useCallback, useEffect, useRef, useState } from 'react'

function clearHighlights(container: HTMLElement | null): void {
  if (!container) return
  const marks = container.querySelectorAll('mark.search-highlight')
  for (const mark of marks) {
    const parent = mark.parentNode!
    parent.replaceChild(document.createTextNode(mark.textContent || ''), mark)
    parent.normalize()
  }
}

function highlightMatches(container: HTMLElement, query: string): number {
  clearHighlights(container)
  if (!query) return 0

  const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT)
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

    const fragment = document.createDocumentFragment()
    let lastEnd = 0
    while (pos !== -1) {
      if (pos > lastEnd) {
        fragment.appendChild(document.createTextNode(text.slice(lastEnd, pos)))
      }
      const mark = document.createElement('mark')
      mark.className = 'search-highlight'
      mark.setAttribute('data-match-index', String(matchIndex++))
      mark.textContent = text.slice(pos, pos + query.length)
      fragment.appendChild(mark)
      lastEnd = pos + query.length
      pos = lowerText.indexOf(lowerQuery, lastEnd)
    }
    if (lastEnd < text.length) {
      fragment.appendChild(document.createTextNode(text.slice(lastEnd)))
    }
    node.parentNode!.replaceChild(fragment, node)
  }

  return matchIndex
}

export function useDocumentSearch(
  containerRef: React.RefObject<HTMLElement | null>,
  query: string,
  htmlVersion: string,
) {
  const [matchCount, setMatchCount] = useState(0)
  const [currentIndex, setCurrentIndex] = useState(-1)
  const prevActiveRef = useRef<Element | null>(null)

  // Run highlighting whenever query or html changes
  useEffect(() => {
    const container = containerRef.current
    if (!container) {
      setMatchCount(0)
      setCurrentIndex(-1)
      return
    }
    const count = highlightMatches(container, query)
    setMatchCount(count)
    setCurrentIndex(count > 0 ? 0 : -1)
  }, [query, htmlVersion, containerRef])

  // Update active highlight styling
  useEffect(() => {
    if (prevActiveRef.current) {
      prevActiveRef.current.classList.remove('search-highlight-active')
    }
    if (currentIndex >= 0) {
      const container = containerRef.current
      if (container) {
        const mark = container.querySelector(`mark[data-match-index="${currentIndex}"]`)
        if (mark) {
          mark.classList.add('search-highlight-active')
          mark.scrollIntoView({ block: 'center', behavior: 'smooth' })
          prevActiveRef.current = mark
        }
      }
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
    clearHighlights(containerRef.current)
    setMatchCount(0)
    setCurrentIndex(-1)
  }, [containerRef])

  return { matchCount, currentIndex, next, prev, clear }
}
