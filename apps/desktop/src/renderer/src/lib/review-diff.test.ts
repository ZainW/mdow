import type { MarkdownDocument, Node } from 'comark'
import { describe, expect, it } from 'vitest'
import { SECTION_TAG } from './markdown-sections'
import { buildReviewTree } from './review-diff'

function doc(...nodes: Node[]): MarkdownDocument {
  return { nodes, frontmatter: {}, meta: {} }
}

const title = (line: number): Node => ['h1', { id: 'plan', $: { line } }, 'Plan']
const para = (text: string, line = 3): Node => ['p', { $: { line } }, text]

describe('buildReviewTree', () => {
  it('leaves an unchanged document alone', () => {
    const result = buildReviewTree(doc(title(1), para('Q4')), doc(title(1), para('Q4')))
    expect(result.changeCount).toBe(0)
    expect(result.tree.nodes).toEqual([title(1), para('Q4')])
  })

  it('pairs a replaced block with its replacement, ignoring shifted line numbers', () => {
    const result = buildReviewTree(
      doc(title(1), para('Teh team'), para('Risks', 5)),
      doc(title(1), para('The team'), para('Risks', 7)),
    )
    expect(result.changeCount).toBe(1)
    expect(result.tree.nodes).toEqual([
      title(1),
      [
        'div',
        { class: 'md-change md-change-modified', 'data-change-index': '0' },
        [
          'div',
          { class: 'md-change-old', role: 'group', 'aria-label': 'Current text' },
          para('Teh team'),
        ],
        [
          'div',
          { class: 'md-change-new', role: 'group', 'aria-label': 'Suggested text' },
          para('The team'),
        ],
      ],
      para('Risks', 7),
    ])
  })

  it('marks pure additions and removals separately', () => {
    const result = buildReviewTree(
      doc(para('a'), para('b'), para('c')),
      doc(para('a'), para('c'), para('d')),
    )
    expect(result.changeCount).toBe(2)
    expect(result.tree.nodes[1]).toEqual([
      'div',
      { class: 'md-change md-change-removed', 'data-change-index': '0' },
      ['div', { class: 'md-change-old', role: 'group', 'aria-label': 'Current text' }, para('b')],
    ])
    expect(result.tree.nodes[3]).toEqual([
      'div',
      { class: 'md-change md-change-added', 'data-change-index': '1' },
      ['div', { class: 'md-change-new', role: 'group', 'aria-label': 'Suggested text' }, para('d')],
    ])
  })

  it('drops anchors from removed headings and unwraps long-document sections', () => {
    const result = buildReviewTree(
      doc([SECTION_TAG, { estimate: 3 }, ['h2', { id: 'old' }, 'Old'], para('x')]),
      doc([SECTION_TAG, { estimate: 3 }, ['h2', { id: 'new' }, 'New'], para('x')]),
    )
    const change = result.tree.nodes[0] as unknown[]
    expect(change[2]).toEqual([
      'div',
      { class: 'md-change-old', role: 'group', 'aria-label': 'Current text' },
      ['h2', {}, 'Old'],
    ])
    expect(result.tree.nodes[1]).toEqual(para('x'))
  })
})

describe('buildReviewTree lists', () => {
  it('marks only the list items that changed', () => {
    const list = (...items: string[]): Node => ['ul', {}, ...items.map((t): Node => ['li', {}, t])]
    const result = buildReviewTree(
      doc(list('Ship', 'Make it usefull', 'Fast', 'Old item')),
      doc(list('Ship', 'Make it useful', 'Fast')),
    )
    expect(result.changeCount).toBe(1)
    expect(result.tree.nodes).toEqual([
      [
        'ul',
        { class: 'md-change md-change-list', 'data-change-index': '0' },
        ['li', {}, 'Ship'],
        ['li', { 'aria-label': 'Current text', class: 'md-change-old' }, 'Make it usefull'],
        ['li', { 'aria-label': 'Suggested text', class: 'md-change-new' }, 'Make it useful'],
        ['li', {}, 'Fast'],
        [
          'li',
          { 'aria-label': 'Current text', class: 'md-change-old md-change-struck' },
          'Old item',
        ],
      ],
    ])
  })
})
