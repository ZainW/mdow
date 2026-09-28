import { describe, expect, it } from 'vitest'
import {
  canOpenDrop,
  describeDrop,
  summarizeDragItems,
  summarizeDroppedEntries,
} from './drop-summary'

describe('summarizeDroppedEntries', () => {
  it('counts markdown files, html files and folders', () => {
    const summary = summarizeDroppedEntries([
      { name: 'a.md', isDirectory: false },
      { name: 'b.markdown', isDirectory: false },
      { name: 'c.MDX', isDirectory: false },
      { name: 'docs', isDirectory: true },
      { name: 'page.html', isDirectory: false },
      { name: 'photo.png', isDirectory: false },
    ])
    expect(summary).toEqual({ markdown: 3, html: 1, folders: 1, unknown: 0, unsupported: 1 })
  })
})

describe('summarizeDragItems', () => {
  it('classifies by MIME type and leaves untyped entries unknown', () => {
    const summary = summarizeDragItems([
      { kind: 'file', type: 'text/markdown' },
      { kind: 'file', type: 'text/x-markdown' },
      { kind: 'file', type: 'text/html' },
      { kind: 'file', type: '' },
      { kind: 'file', type: 'image/png' },
      { kind: 'string', type: 'text/plain' },
    ])
    expect(summary).toEqual({ markdown: 2, html: 1, folders: 0, unknown: 1, unsupported: 1 })
  })
})

describe('describeDrop', () => {
  it('joins counts with a middle dot', () => {
    expect(describeDrop({ markdown: 3, html: 0, folders: 1, unknown: 0, unsupported: 2 })).toBe(
      '3 Markdown files · 1 folder',
    )
  })

  it('uses singular nouns for one', () => {
    expect(describeDrop({ markdown: 1, html: 1, folders: 0, unknown: 2, unsupported: 0 })).toBe(
      '1 Markdown file · 1 HTML file · 2 items',
    )
  })

  it('says when nothing can be opened', () => {
    const summary = { markdown: 0, html: 0, folders: 0, unknown: 0, unsupported: 3 }
    expect(canOpenDrop(summary)).toBe(false)
    expect(describeDrop(summary)).toBe('Nothing Mdow can open')
  })
})
