function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

export function findMatchRanges(
  text: string,
  query: string,
): ReadonlyArray<{ start: number; end: number }> {
  if (!query) return []
  const regex = new RegExp(escapeRegExp(query), 'gi')
  const ranges: { start: number; end: number }[] = []
  for (const match of text.matchAll(regex)) {
    if (match.index !== undefined) {
      ranges.push({ start: match.index, end: match.index + match[0].length })
    }
  }
  return ranges
}

export function shouldSkipSearchTextNode(node: Text): boolean {
  const parent = node.parentElement
  return (
    !parent || Boolean(parent.closest('mark.search-highlight, .copy-code-btn, .mermaid-container'))
  )
}

export function applySearchHighlights(container: HTMLElement, query: string): number {
  if (!query) return 0

  const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      return node instanceof Text && !shouldSkipSearchTextNode(node)
        ? NodeFilter.FILTER_ACCEPT
        : NodeFilter.FILTER_REJECT
    },
  })

  const textNodes: Text[] = []
  while (walker.nextNode()) {
    const node = walker.currentNode
    if (node instanceof Text) textNodes.push(node)
  }

  let matchIndex = 0

  for (const node of textNodes) {
    const text = node.textContent || ''
    const ranges = findMatchRanges(text, query)
    if (ranges.length === 0) continue

    const fragment = document.createDocumentFragment()
    let lastEnd = 0
    for (const { start, end } of ranges) {
      if (start > lastEnd) {
        fragment.appendChild(document.createTextNode(text.slice(lastEnd, start)))
      }
      const mark = document.createElement('mark')
      mark.className = 'search-highlight'
      mark.setAttribute('data-match-index', String(matchIndex++))
      mark.textContent = text.slice(start, end)
      fragment.appendChild(mark)
      lastEnd = end
    }
    if (lastEnd < text.length) {
      fragment.appendChild(document.createTextNode(text.slice(lastEnd)))
    }
    node.parentNode!.replaceChild(fragment, node)
  }

  return matchIndex
}

export function removeSearchHighlights(container: HTMLElement): void {
  for (const mark of container.querySelectorAll('mark.search-highlight')) {
    const parent = mark.parentNode
    if (!parent) continue
    parent.replaceChild(document.createTextNode(mark.textContent ?? ''), mark)
    parent.normalize()
  }
}
