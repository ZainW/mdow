import { describe, it, expect } from 'vitest'
import { parseMarkdown } from './parser'
import { serializeMarkdown } from './serializer'

describe('serializeMarkdown', () => {
  it('serializes a heading', () => {
    const doc = parseMarkdown('# Hello')
    expect(serializeMarkdown(doc)).toBe('# Hello')
  })

  it('serializes a paragraph with bold', () => {
    const doc = parseMarkdown('This is **bold** text')
    expect(serializeMarkdown(doc)).toBe('This is **bold** text')
  })

  it('serializes a code block with language', () => {
    const doc = parseMarkdown('```ts\nconst x = 1\n```')
    expect(serializeMarkdown(doc)).toBe('```ts\nconst x = 1\n```')
  })
})
