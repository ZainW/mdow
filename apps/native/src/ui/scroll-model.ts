import type { Block } from '../lib/markdown'

/**
 * Rough pixel heights for every row, so a scrollbar can place a document that is mostly
 * unmeasured. Only rows near the viewport are ever laid out; these estimates stand in for the
 * rest and the exact in-row offset GPUI reports keeps the thumb moving smoothly inside a row.
 */
export function estimateHeights(blocks: Block[], columnWidth: number, fontSize: number): number[] {
  const line = fontSize * 1.75
  const codeLine = Math.round(fontSize * 0.875 * 1.6)
  const charsPerLine = Math.max(20, columnWidth / (fontSize * 0.5))
  const prose = (text: string) =>
    text
      .split('\n')
      .reduce((sum, part) => sum + Math.max(1, Math.ceil(part.length / charsPerLine)), 0) * line
  return blocks.map((block, index) => {
    const gap = index === blocks.length - 1 ? 40 : block.joinNext ? 0 : fontSize
    switch (block.kind) {
      case 'heading':
        return fontSize * (block.level === 1 ? 2.5 : block.level === 2 ? 2 : 1.6) + gap
      case 'code': {
        const lines = block.code.split('\n').length
        const chrome = block.part === 'whole' ? 44 : block.part === 'middle' ? 0 : 22
        return block.language === 'mermaid' ? 320 + gap : lines * codeLine + chrome + gap
      }
      case 'image':
        return (
          (block.src ? Math.min(block.height, (block.height / block.width) * columnWidth) : line) +
          gap
        )
      case 'rule':
        return fontSize + 1 + gap
      case 'table':
        return (block.rows.length + (block.header ? 1 : 0)) * (line + fontSize * 0.75) + gap
      default:
        return prose(block.text) + gap
    }
  })
}

export function prefixSums(heights: number[]): number[] {
  const sums = [0]
  for (const height of heights) sums.push(sums.at(-1)! + height)
  return sums
}

/** Row and in-row offset at an estimated document y. */
export function rowAt(sums: number[], y: number): [number, number] {
  let low = 0
  let high = sums.length - 2
  while (low < high) {
    const mid = (low + high + 1) >> 1
    if (sums[mid]! <= y) low = mid
    else high = mid - 1
  }
  return [Math.max(0, low), Math.max(0, y - (sums[Math.max(0, low)] ?? 0))]
}
