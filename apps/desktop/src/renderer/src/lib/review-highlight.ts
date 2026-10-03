// Word-level marks for suggested edits, painted with the CSS Custom Highlight API like search
// matches: the rendered blocks are compared as text and the differences become Ranges, so the
// document DOM is never modified.
import { shouldSkipSearchTextNode } from './search-highlight'
import { diffWords, type TextSpan } from './word-diff'

const INSERTED_HIGHLIGHT = 'mdow-diff-inserted'
const REMOVED_HIGHLIGHT = 'mdow-diff-removed'

interface TextMap {
  text: string
  nodes: { node: Text; start: number }[]
}

function collectText(root: Element): TextMap {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      return node instanceof Text && !shouldSkipSearchTextNode(node)
        ? NodeFilter.FILTER_ACCEPT
        : NodeFilter.FILTER_REJECT
    },
  })
  const nodes: TextMap['nodes'] = []
  let text = ''
  while (walker.nextNode()) {
    const node = walker.currentNode as Text
    nodes.push({ node, start: text.length })
    text += node.data
  }
  return { text, nodes }
}

function locate(map: TextMap, offset: number, preferEnd: boolean): [Text, number] | null {
  for (let index = 0; index < map.nodes.length; index += 1) {
    const { node, start } = map.nodes[index]
    const end = start + node.data.length
    if (offset < end || (preferEnd && offset === end)) return [node, offset - start]
  }
  const last = map.nodes.at(-1)
  return last ? [last.node, last.node.data.length] : null
}

function rangesFor(map: TextMap, spans: TextSpan[]): Range[] {
  return spans.flatMap(({ start, end }) => {
    const from = locate(map, start, false)
    const to = locate(map, end, true)
    if (!from || !to) return []
    const range = document.createRange()
    range.setStart(from[0], from[1])
    range.setEnd(to[0], to[1])
    return [range]
  })
}

function highlightRegistry(): HighlightRegistry | null {
  return typeof CSS !== 'undefined' && 'highlights' in CSS && typeof Highlight !== 'undefined'
    ? CSS.highlights
    : null
}

/**
 * Replaced blocks (or list items) come as a run of old elements followed by a run of new ones;
 * they are paired by position, so the first old item is compared with the first new one.
 */
export function replacementPairs(container: Element): [Element, Element][] {
  const pairs: [Element, Element][] = []
  for (const first of container.querySelectorAll('.md-change-old')) {
    if (first.previousElementSibling?.classList.contains('md-change-old')) continue
    const olds: Element[] = []
    let cursor: Element | null = first
    while (cursor?.classList.contains('md-change-old')) {
      olds.push(cursor)
      cursor = cursor.nextElementSibling
    }
    const news: Element[] = []
    while (cursor?.classList.contains('md-change-new')) {
      news.push(cursor)
      cursor = cursor.nextElementSibling
    }
    for (let index = 0; index < Math.min(olds.length, news.length); index += 1) {
      pairs.push([olds[index], news[index]])
    }
  }
  return pairs
}

/** Marks inserted and removed words in every replaced block under `container`. */
export function paintReviewHighlights(container: Element): void {
  const registry = highlightRegistry()
  if (!registry) return
  const inserted: Range[] = []
  const removed: Range[] = []
  for (const [before, after] of replacementPairs(container)) {
    const oldText = collectText(before)
    const newText = collectText(after)
    const diff = diffWords(oldText.text, newText.text)
    removed.push(...rangesFor(oldText, diff.removed))
    inserted.push(...rangesFor(newText, diff.inserted))
  }
  registry.set(INSERTED_HIGHLIGHT, new Highlight(...inserted))
  registry.set(REMOVED_HIGHLIGHT, new Highlight(...removed))
}

export function clearReviewHighlights(): void {
  const registry = highlightRegistry()
  registry?.delete(INSERTED_HIGHLIGHT)
  registry?.delete(REMOVED_HIGHLIGHT)
}
