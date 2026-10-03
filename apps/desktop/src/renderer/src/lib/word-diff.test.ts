import { describe, expect, it } from 'vitest'
import { diffWords } from './word-diff'

function pick(text: string, spans: { start: number; end: number }[]) {
  return spans.map(({ start, end }) => text.slice(start, end))
}

describe('diffWords', () => {
  it('finds changed words on each side', () => {
    const before = 'Teh team is small, so we recieve feedback slowly.'
    const after = 'The team is small, so we receive feedback slowly.'
    const diff = diffWords(before, after)
    expect(pick(before, diff.removed)).toEqual(['Teh', 'recieve'])
    expect(pick(after, diff.inserted)).toEqual(['The', 'receive'])
  })

  it('merges neighbouring changes into one span', () => {
    const before = 'We ship in Q4 next year.'
    const after = 'We ship early in Q1 next year.'
    const diff = diffWords(before, after)
    expect(pick(after, diff.inserted)).toEqual(['early', 'Q1'])
    expect(pick(before, diff.removed)).toEqual(['Q4'])
  })

  it('reports nothing for identical text and everything for disjoint text', () => {
    expect(diffWords('same words', 'same words')).toEqual({ removed: [], inserted: [] })
    const diff = diffWords('alpha beta', 'gamma delta')
    expect(pick('alpha beta', diff.removed)).toEqual(['alpha beta'])
    expect(pick('gamma delta', diff.inserted)).toEqual(['gamma delta'])
  })
})
