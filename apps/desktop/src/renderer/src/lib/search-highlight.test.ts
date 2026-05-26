import { describe, expect, it } from 'vitest'
import {
  applySearchHighlights,
  findMatchRanges,
  removeSearchHighlights,
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

describe('shouldSkipSearchTextNode', () => {
  it('skips text inside existing search highlights', () => {
    const container = document.createElement('div')
    const mark = document.createElement('mark')
    mark.className = 'search-highlight'
    mark.textContent = 'skip me'
    container.appendChild(mark)
    const node = mark.firstChild as Text
    expect(shouldSkipSearchTextNode(node)).toBe(true)
  })

  it('skips text inside copy-code buttons', () => {
    const container = document.createElement('div')
    const button = document.createElement('button')
    button.className = 'copy-code-btn'
    button.textContent = 'Copy'
    container.appendChild(button)
    const node = button.firstChild as Text
    expect(shouldSkipSearchTextNode(node)).toBe(true)
  })

  it('skips text inside mermaid containers', () => {
    const container = document.createElement('div')
    const diagram = document.createElement('div')
    diagram.className = 'mermaid-container'
    diagram.textContent = 'graph TD'
    container.appendChild(diagram)
    const node = diagram.firstChild as Text
    expect(shouldSkipSearchTextNode(node)).toBe(true)
  })

  it('accepts normal text nodes', () => {
    const container = document.createElement('div')
    container.textContent = 'searchable text'
    const node = container.firstChild as Text
    expect(shouldSkipSearchTextNode(node)).toBe(false)
  })
})

describe('applySearchHighlights / removeSearchHighlights', () => {
  it('wraps matches in mark elements and returns the count', () => {
    const container = document.createElement('div')
    const paragraph = document.createElement('p')
    paragraph.textContent = 'hello world hello'
    container.appendChild(paragraph)

    const count = applySearchHighlights(container, 'hello')

    expect(count).toBe(2)
    expect(container.querySelectorAll('mark.search-highlight')).toHaveLength(2)
    expect(container.querySelectorAll('mark.search-highlight')[0].textContent).toBe('hello')
  })

  it('skips nodes that should not be highlighted', () => {
    const container = document.createElement('div')
    const paragraph = document.createElement('p')
    paragraph.textContent = 'visible'
    const mark = document.createElement('mark')
    mark.className = 'search-highlight'
    mark.textContent = 'hidden'
    container.append(paragraph, mark)

    applySearchHighlights(container, 'hidden')

    expect(container.querySelectorAll('mark.search-highlight')).toHaveLength(1)
  })

  it('removeSearchHighlights unwraps marks back to text', () => {
    const container = document.createElement('div')
    const paragraph = document.createElement('p')
    paragraph.textContent = 'hello world'
    container.appendChild(paragraph)

    applySearchHighlights(container, 'hello')
    removeSearchHighlights(container)

    expect(container.querySelectorAll('mark.search-highlight')).toHaveLength(0)
    expect(container.textContent).toBe('hello world')
  })
})
