import { describe, expect, it } from 'vitest'
import { applySearchHighlights } from '../lib/search-highlight'

function makeSearchableDom(sectionCount: number): HTMLElement {
  const root = document.createElement('div')
  for (let i = 0; i < sectionCount; i++) {
    const p = document.createElement('p')
    p.textContent = `Section ${i} with searchable performance content repeated many times.`
    root.appendChild(p)
  }
  return root
}

describe('document search highlight performance', () => {
  it('highlights a large document within the typing budget', () => {
    const container = makeSearchableDom(240)
    const startedAt = performance.now()

    const count = applySearchHighlights(container, 'performance')

    expect(performance.now() - startedAt).toBeLessThan(120)
    expect(count).toBeGreaterThan(200)
  })
})
