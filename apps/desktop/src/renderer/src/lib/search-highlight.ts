// In-document search paints matches with the CSS Custom Highlight API: matches are Ranges, and
// the DOM is never modified. That keeps search compatible with React re-rendering blocks under
// it (lazy syntax highlighting, live reload) and makes clearing a search free.

const MATCH_HIGHLIGHT = 'mdow-search'
const ACTIVE_HIGHLIGHT = 'mdow-search-active'
const SKIP_SELECTOR = '.copy-code-btn, .code-lang-badge, .mermaid-container'

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
  return !parent || Boolean(parent.closest(SKIP_SELECTOR))
}

/** Every match of `query` in `container`'s text, in document order. */
export function findSearchRanges(container: HTMLElement, query: string): Range[] {
  if (!query) return []

  const regex = new RegExp(escapeRegExp(query), 'gi')
  const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      return node instanceof Text && !shouldSkipSearchTextNode(node)
        ? NodeFilter.FILTER_ACCEPT
        : NodeFilter.FILTER_REJECT
    },
  })

  const ranges: Range[] = []
  while (walker.nextNode()) {
    const node = walker.currentNode
    const text = node.nodeValue
    if (!text) continue
    regex.lastIndex = 0
    for (const match of text.matchAll(regex)) {
      const range = document.createRange()
      range.setStart(node, match.index)
      range.setEnd(node, match.index + match[0].length)
      ranges.push(range)
    }
  }
  return ranges
}

function highlightRegistry(): HighlightRegistry | null {
  return typeof CSS !== 'undefined' && 'highlights' in CSS && typeof Highlight !== 'undefined'
    ? CSS.highlights
    : null
}

export function paintSearchHighlights(ranges: readonly Range[], activeIndex: number): void {
  const registry = highlightRegistry()
  if (!registry) return
  registry.set(MATCH_HIGHLIGHT, new Highlight(...ranges))
  const active = ranges[activeIndex]
  if (active) {
    const highlight = new Highlight(active)
    highlight.priority = 1
    registry.set(ACTIVE_HIGHLIGHT, highlight)
  } else {
    registry.delete(ACTIVE_HIGHLIGHT)
  }
}

export function clearSearchHighlights(): void {
  const registry = highlightRegistry()
  registry?.delete(MATCH_HIGHLIGHT)
  registry?.delete(ACTIVE_HIGHLIGHT)
}
