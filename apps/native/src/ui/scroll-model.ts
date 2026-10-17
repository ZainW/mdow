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
      case 'markdown':
      case 'quote':
      case 'alert':
        return prose(block.source) + gap
      case 'footnotes':
        return prose(block.items.map((note) => `${note.marker} ${note.source}`).join('\n')) + gap
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

/**
 * The reader mounts rows by estimated height rather than by count: one slice of a big table or
 * code fence weighs as much as a page of prose, and a fixed count of them mounted every cell of a
 * 1,500-row table. These are estimated pixels kept above the first visible row and in total.
 */
export const WINDOW_BEHIND = 2_500
export const WINDOW_SPAN = 9_000
/** The window moves once the viewport comes within this many pixels of either end of it. */
export const WINDOW_EDGE = 1_500
/** Never more rows than this, however short they are. */
export const WINDOW_MAX_ROWS = 160

/** First mounted row for a viewport whose first visible row is `row`. */
export function windowStartFor(sums: number[], row: number): number {
  const count = sums.length - 1
  if (count <= 0) return 0
  const target = Math.max(0, Math.min(row, count))
  const [byHeight] = rowAt(sums, Math.max(0, sums[target]! - WINDOW_BEHIND))
  // Short rows stop at the row cap, so keep most of the window ahead of the viewport.
  return Math.min(count - 1, Math.max(byHeight, target - Math.floor(WINDOW_MAX_ROWS / 4)))
}

/** One past the last mounted row of a window starting at `start`. */
export function windowEndFor(sums: number[], start: number): number {
  const count = sums.length - 1
  if (count <= 0) return 0
  const [last] = rowAt(sums, sums[start]! + WINDOW_SPAN)
  return Math.max(start + 1, Math.min(count, last + 1, start + WINDOW_MAX_ROWS))
}

/** Whether visible rows `first`…`last` sit near an edge of the mounted rows `start`…`end`. */
export function needsRewindow(
  sums: number[],
  start: number,
  end: number,
  first: number,
  last: number,
): boolean {
  const count = sums.length - 1
  if (first < start || last >= end) return true
  const nearTop = start > 0 && sums[first]! - sums[start]! < WINDOW_EDGE
  const nearBottom = end < count && sums[end]! - sums[Math.min(count, last + 1)]! < WINDOW_EDGE
  return nearTop || nearBottom
}
