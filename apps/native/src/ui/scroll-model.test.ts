import { describe, expect, test } from 'bun:test'
import { parseMarkdown } from '../lib/markdown'
import { easeOut } from '../lib/motion'
import { estimateHeights, prefixSums, rowAt } from './scroll-model'

describe('scroll model', () => {
  test('rowAt maps a document y back to a row and offset', () => {
    const sums = prefixSums([100, 50, 200])
    expect(sums).toEqual([0, 100, 150, 350])
    expect(rowAt(sums, 0)).toEqual([0, 0])
    expect(rowAt(sums, 120)).toEqual([1, 20])
    expect(rowAt(sums, 349)).toEqual([2, 199])
  })

  test('taller content estimates taller', () => {
    const doc = parseMarkdown(
      `# Title\n\nShort.\n\n${'Long paragraph words. '.repeat(80)}\n`,
      '/a.md',
    )
    const [heading, short, long] = estimateHeights(doc.blocks, 700, 16)
    expect(long!).toBeGreaterThan(short!)
    expect(heading!).toBeGreaterThan(0)
  })

  test('easeOut starts fast and lands at 1', () => {
    expect(easeOut(0)).toBe(0)
    expect(easeOut(1)).toBe(1)
    expect(easeOut(0.25)).toBeGreaterThan(0.25)
  })
})
