import { useCallback, useEffect, useRef, useState, type RefObject } from 'react'

function removeHighlights(container: HTMLElement): void {
  for (const mark of container.querySelectorAll('mark.search-highlight')) {
    const parent = mark.parentNode
    if (!parent) continue
    parent.replaceChild(document.createTextNode(mark.textContent ?? ''), mark)
    parent.normalize()
  }
}

function shouldSkipTextNode(node: Text): boolean {
  const parent = node.parentElement
  return (
    !parent || Boolean(parent.closest('mark.search-highlight, .copy-code-btn, .mermaid-container'))
  )
}

function applyHighlights(container: HTMLElement, query: string): number {
  if (!query) return 0

  const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      return node instanceof Text && !shouldSkipTextNode(node)
        ? NodeFilter.FILTER_ACCEPT
        : NodeFilter.FILTER_REJECT
    },
  })

  const textNodes: Text[] = []
  while (walker.nextNode()) {
    const node = walker.currentNode
    if (node instanceof Text) textNodes.push(node)
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
  containerRef: RefObject<HTMLElement | null>,
  query: string,
  renderKey: unknown,
) {
  const [currentIndex, setCurrentIndex] = useState(-1)
  const [matchCount, setMatchCount] = useState(0)
  const activeMarkRef = useRef<Element | null>(null)

  useEffect(() => {
    const container = containerRef.current
    if (!container) return undefined

    const timer = setTimeout(() => {
      removeHighlights(container)
      activeMarkRef.current = null

      const count = applyHighlights(container, query)
      setMatchCount(count)
      setCurrentIndex(count > 0 ? 0 : -1)
    }, 120)

    return () => {
      clearTimeout(timer)
      removeHighlights(container)
    }
  }, [containerRef, query, renderKey])

  useEffect(() => {
    const container = containerRef.current
    if (!container) return

    if (activeMarkRef.current?.isConnected) {
      activeMarkRef.current.classList.remove('search-highlight-active')
    }
    activeMarkRef.current = null

    if (currentIndex < 0) return

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
    const container = containerRef.current
    if (container) removeHighlights(container)
    setMatchCount(0)
    setCurrentIndex(-1)
    activeMarkRef.current = null
  }, [containerRef])

  return { matchCount, currentIndex, next, prev, clear }
}
