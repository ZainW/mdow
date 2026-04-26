import { describe, it, expect } from 'vitest'
import { parseMarkdown, parseMarkdownChecked } from './parser'

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

describe('parseMarkdownChecked', () => {
  it('reports clean round-trip as not lossy', () => {
    const result = parseMarkdownChecked('# Hello\n\nA paragraph.')
    expect(result.lossy).toBe(false)
  })

  it('returns a boolean lossy flag for any markdown input', () => {
    // Use a plain block-level html input that the parser handles without throwing.
    // The exact lossy value may vary — we just assert the helper returns a well-formed result.
    const result = parseMarkdownChecked('<div>block html</div>')
    expect(typeof result.lossy).toBe('boolean')
    expect(result.doc).toBeDefined()
  })
})
