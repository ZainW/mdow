import { describe, it, expect } from 'vitest'
import { parseMarkdown } from './parser'

describe('parseMarkdown', () => {
  it('parses a heading', () => {
    const doc = parseMarkdown('# Hello')
    expect(doc.firstChild?.type.name).toBe('heading')
    expect(doc.firstChild?.attrs.level).toBe(1)
    expect(doc.firstChild?.textContent).toBe('Hello')
  })

  it('parses a paragraph with bold', () => {
    const doc = parseMarkdown('This is **bold** text')
    expect(doc.firstChild?.type.name).toBe('paragraph')
    expect(doc.textContent).toBe('This is bold text')
  })

  it('parses a code block', () => {
    const doc = parseMarkdown('```ts\nconst x = 1\n```')
    expect(doc.firstChild?.type.name).toBe('code_block')
    expect(doc.firstChild?.attrs.params).toBe('ts')
  })
})
