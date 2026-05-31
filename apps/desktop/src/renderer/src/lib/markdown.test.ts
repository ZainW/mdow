import { describe, expect, it } from 'vitest'
import { initMarkdown, renderMarkdown } from './markdown'

describe('renderMarkdown', () => {
  it('extracts headings h1 through h6 with slug ids', async () => {
    await initMarkdown()
    const markdown = [
      '# Alpha',
      '## Beta',
      '### Gamma',
      '#### Delta',
      '##### Epsilon',
      '###### Zeta',
    ].join('\n')

    const result = await renderMarkdown(markdown)

    expect(result.headings).toHaveLength(6)
    expect(result.headings.map((heading) => heading.level)).toEqual([1, 2, 3, 4, 5, 6])
    expect(result.headings.map((heading) => heading.text)).toEqual([
      'Alpha',
      'Beta',
      'Gamma',
      'Delta',
      'Epsilon',
      'Zeta',
    ])
    expect(result.headings.every((heading) => heading.id.length > 0)).toBe(true)
  })

  it('deduplicates slug ids for repeated headings', async () => {
    await initMarkdown()
    const markdown = ['# Repeat', '## Repeat', '# Repeat'].join('\n')

    const result = await renderMarkdown(markdown)

    expect(result.headings.map((heading) => heading.id)).toEqual(['repeat', 'repeat-1', 'repeat-2'])
  })

  it('returns parsed frontmatter', async () => {
    await initMarkdown()
    const markdown = ['---', 'title: Hello', 'tags:', '  - docs', '---', '', '# Body'].join('\n')

    const result = await renderMarkdown(markdown)

    expect(result.frontmatter).toEqual({ title: 'Hello', tags: ['docs'] })
    expect(result.headings).toEqual([{ level: 1, text: 'Body', id: 'body' }])
  })

  it('collects mermaid diagram blocks', async () => {
    await initMarkdown()
    const markdown = ['```mermaid', 'flowchart TD', '  A --> B', '```'].join('\n')

    const result = await renderMarkdown(markdown)

    expect(result.mermaidBlocks).toHaveLength(1)
    expect(result.mermaidBlocks[0]?.code).toContain('flowchart TD')
    expect(result.mermaidBlocks[0]?.id).toMatch(/^mermaid-/)
  })

  it('reuses cached render results for unchanged content', async () => {
    await initMarkdown()
    const markdown = '# Cached'

    const first = await renderMarkdown(markdown)
    const second = await renderMarkdown(markdown)

    expect(second).toBe(first)
  })

  it('parses inline math when math plugin is enabled', async () => {
    await initMarkdown()
    const result = await renderMarkdown('Inline $x^2$ math')

    const serialized = JSON.stringify(result.tree.nodes)
    expect(serialized).toContain('math')
    expect(serialized).toContain('x^2')
  })
})
