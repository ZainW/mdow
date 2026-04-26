import { describe, it, expect } from 'vitest'
import { parseMarkdown } from '../parser'
import { serializeMarkdown } from '../serializer'

describe('mermaid block', () => {
  it('parses ```mermaid into a mermaidBlock node', () => {
    const doc = parseMarkdown('```mermaid\ngraph TD\n  A --> B\n```')
    expect(doc.firstChild?.type.name).toBe('mermaidBlock')
    expect(doc.firstChild?.attrs.source).toBe('graph TD\n  A --> B')
  })

  it('round-trips a mermaid block', () => {
    const input = '```mermaid\ngraph TD\n  A --> B\n```'
    const doc = parseMarkdown(input)
    expect(serializeMarkdown(doc)).toBe(input)
  })
})
