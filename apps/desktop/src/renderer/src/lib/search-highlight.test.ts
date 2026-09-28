import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  clearSearchHighlights,
  findMatchRanges,
  findSearchRanges,
  paintSearchHighlights,
  shouldSkipSearchTextNode,
} from './search-highlight'

describe('findMatchRanges', () => {
  it('returns empty array for empty query', () => {
    expect(findMatchRanges('hello world', '')).toEqual([])
  })

  it('finds case-insensitive matches', () => {
    expect(findMatchRanges('Hello HELLO hello', 'hello')).toEqual([
      { start: 0, end: 5 },
      { start: 6, end: 11 },
      { start: 12, end: 17 },
    ])
  })

  it('escapes regex special characters in the query', () => {
    expect(findMatchRanges('a.b (test)', 'a.b')).toEqual([{ start: 0, end: 3 }])
  })
})

function textIn(element: HTMLElement): Text {
  const node = element.firstChild
  if (!(node instanceof Text)) throw new Error('expected a text node')
  return node
}

describe('shouldSkipSearchTextNode', () => {
  it.each([
    'code-block-header',
    'copy-code-btn',
    'code-lang-badge',
    'markdown-alert-title',
    'mermaid-container',
  ])('skips text in .%s', (cls) => {
    const element = document.createElement('div')
    element.className = cls
    element.textContent = 'skip me'
    expect(shouldSkipSearchTextNode(textIn(element))).toBe(true)
  })

  it('accepts normal text nodes', () => {
    const container = document.createElement('div')
    container.textContent = 'searchable text'
    expect(shouldSkipSearchTextNode(textIn(container))).toBe(false)
  })
})

describe('findSearchRanges', () => {
  it('returns one range per match without touching the DOM', () => {
    const container = document.createElement('div')
    container.innerHTML = '<p>hello world hello</p><p>say <strong>hello</strong></p>'
    const before = container.innerHTML

    const ranges = findSearchRanges(container, 'hello')

    expect(ranges).toHaveLength(3)
    expect(ranges.map((range) => range.toString())).toEqual(['hello', 'hello', 'hello'])
    expect(container.innerHTML).toBe(before)
  })

  it('skips excluded subtrees', () => {
    const container = document.createElement('div')
    container.innerHTML = '<p>ts code</p><span class="code-lang-badge">ts</span>'
    expect(findSearchRanges(container, 'ts')).toHaveLength(1)
  })

  it('returns nothing for an empty query', () => {
    const container = document.createElement('div')
    container.textContent = 'anything'
    expect(findSearchRanges(container, '')).toEqual([])
  })
})

describe('paintSearchHighlights', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('registers match and active highlights when the API exists', () => {
    const registry = new Map<string, unknown>()
    class FakeHighlight {
      ranges: Range[]
      priority = 0
      constructor(...ranges: Range[]) {
        this.ranges = ranges
      }
    }
    vi.stubGlobal('CSS', { highlights: registry })
    vi.stubGlobal('Highlight', FakeHighlight)

    const container = document.createElement('div')
    container.textContent = 'a b a'
    const ranges = findSearchRanges(container, 'a')
    paintSearchHighlights(ranges, 1)

    expect((registry.get('mdow-search') as FakeHighlight).ranges).toHaveLength(2)
    expect((registry.get('mdow-search-active') as FakeHighlight).ranges).toEqual([ranges[1]])

    clearSearchHighlights()
    expect(registry.size).toBe(0)
  })

  it('is a no-op without the Highlight API', () => {
    expect(() => paintSearchHighlights([], 0)).not.toThrow()
  })
})
