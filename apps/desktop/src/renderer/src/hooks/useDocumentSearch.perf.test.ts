import { describe, expect, it } from 'vitest'

function makeSearchableDom(sectionCount: number): HTMLElement {
  const root = document.createElement('div')
  for (let i = 0; i < sectionCount; i++) {
    const p = document.createElement('p')
    p.textContent = `Section ${i} with searchable performance content repeated many times.`
    root.appendChild(p)
  }
  return root
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

describe('document search highlight performance', () => {
  it('highlights a large document within the typing budget', () => {
    const container = makeSearchableDom(240)
    const startedAt = performance.now()

    const count = applyHighlights(container, 'performance')

    expect(performance.now() - startedAt).toBeLessThan(120)
    expect(count).toBeGreaterThan(200)
  })
})
