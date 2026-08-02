import { describe, expect, it } from 'vitest'
import { selectInitialMarkdown } from './context-selection'

describe('Companion markdown selection', () => {
  it('returns a small markdown document whole', () => {
    const selected = selectInitialMarkdown('# Intro\nComplete body', 'summarize')

    expect(selected.wholeDocument).toBe(true)
    expect(selected.excerpt).toContain('Complete body')
  })

  it('selects question-relevant sections from a large document', () => {
    const largeMarkdown = [
      '# Handbook',
      '',
      '## Background',
      'x'.repeat(20_000),
      '',
      '## Authentication',
      'Use short-lived access tokens and rotate refresh tokens.',
      '',
      '## Appendix',
      'y'.repeat(20_000),
    ].join('\n')

    const selected = selectInitialMarkdown(largeMarkdown, 'How does authentication work?')

    expect(selected.wholeDocument).toBe(false)
    expect(selected.excerpt).toContain('## Authentication')
    expect(selected.excerpt).toContain('short-lived access tokens')
    expect(selected.excerpt).not.toContain('x'.repeat(20_000))
    expect(selected.bytes).toBeLessThanOrEqual(16_384)
  })

  it('extracts local markdown links as metadata without resolving targets', () => {
    const selected = selectInitialMarkdown(
      '[API guide](./api.md) and [Setup][setup].\n\n[setup]: ../setup.md',
      'summarize',
    )

    expect(selected.links).toEqual([
      { label: 'API guide', target: './api.md' },
      { label: 'Setup', target: '../setup.md' },
    ])
  })

  it('excludes remote, mail, and in-document links from the local manifest', () => {
    const selected = selectInitialMarkdown(
      '[Web](https://example.com) [Mail](mailto:test@example.com) [Heading](#intro)',
      'summarize',
    )

    expect(selected.links).toEqual([])
  })
})
