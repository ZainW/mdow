import { describe, it, expect } from 'vitest'
import { parseMarkdown } from '../parser'
import { serializeMarkdown } from '../serializer'

describe('frontmatter passthrough', () => {
  it('extracts frontmatter into a frontmatter node', () => {
    const input = '---\ntitle: Hi\n---\n\n# Body'
    const doc = parseMarkdown(input)
    expect(doc.firstChild?.type.name).toBe('frontmatter')
    expect(doc.firstChild?.attrs.source).toBe('title: Hi')
  })

  it('round-trips frontmatter', () => {
    const input = '---\ntitle: Hi\n---\n\n# Body'
    expect(serializeMarkdown(parseMarkdown(input))).toBe(input)
  })
})

describe('html passthrough', () => {
  it('preserves a raw HTML block', () => {
    const input = 'Para.\n\n<div class="x">y</div>\n\nMore.'
    expect(serializeMarkdown(parseMarkdown(input))).toBe(input)
  })
})
