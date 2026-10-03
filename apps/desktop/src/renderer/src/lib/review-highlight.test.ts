import { describe, expect, it } from 'vitest'
import { replacementPairs } from './review-highlight'

describe('replacementPairs', () => {
  it('pairs runs of replaced items with their replacements by position', () => {
    const list = document.createElement('ul')
    list.innerHTML = [
      '<li>same</li>',
      '<li class="md-change-old">a</li>',
      '<li class="md-change-old">b</li>',
      '<li class="md-change-new">A</li>',
      '<li class="md-change-new">B</li>',
      '<li class="md-change-new">C</li>',
      '<li class="md-change-old md-change-struck">gone</li>',
    ].join('')
    expect(replacementPairs(list).map(([o, n]) => `${o.textContent}>${n.textContent}`)).toEqual([
      'a>A',
      'b>B',
    ])
  })
})
