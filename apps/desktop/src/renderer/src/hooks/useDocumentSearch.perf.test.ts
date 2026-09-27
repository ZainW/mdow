import { describe, expect, it } from 'vitest'
import { findSearchRanges } from '../lib/search-highlight'

function makeSearchableDom(sectionCount: number): HTMLElement {
  const root = document.createElement('div')
  for (let i = 0; i < sectionCount; i++) {
    const p = document.createElement('p')
    p.textContent = `Section ${i} with searchable performance content repeated many times.`
    root.appendChild(p)
  }
  return root
}

describe('document search performance', () => {
  it('finds matches in a large document within the typing budget', () => {
    const container = makeSearchableDom(2_400)
    const startedAt = performance.now()

    const ranges = findSearchRanges(container, 'performance')

    expect(performance.now() - startedAt).toBeLessThan(120)
    expect(ranges).toHaveLength(2_400)
  })
})
