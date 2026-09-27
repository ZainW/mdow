import { describe, expect, it } from 'vitest'
import { groupIntoSections, initMarkdown, renderMarkdown } from './markdown'
import { SECTION_TAG } from './markdown-sections'

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

  it('keeps short documents flat', async () => {
    const result = await renderMarkdown('# One\n\nText\n\n## Two\n\nMore', { bypassCache: true })
    expect(result.tree.nodes.some((node) => Array.isArray(node) && node[0] === SECTION_TAG)).toBe(
      false,
    )
  })

  it('groups long documents into heading-aligned sections without losing blocks', async () => {
    const markdown = Array.from(
      { length: 120 },
      (_, i) => `## Part ${i}\n\nParagraph ${i}.\n\n- item`,
    ).join('\n\n')
    const result = await renderMarkdown(markdown, { bypassCache: true })
    const sections = result.tree.nodes.filter(
      (node) => Array.isArray(node) && node[0] === SECTION_TAG,
    ) as unknown as [string, Record<string, unknown>, ...unknown[]][]

    expect(sections.length).toBe(result.tree.nodes.length)
    expect(sections.length).toBeGreaterThan(1)
    for (const section of sections) {
      expect((section[2] as unknown[])[0]).toBe('h2')
      expect(section[1].estimate).toBeGreaterThan(0)
    }
    const blockCount = sections.reduce((sum, section) => sum + section.length - 2, 0)
    expect(blockCount).toBe(360)
    expect(result.headings).toHaveLength(120)
  })

  it('splits a heading-less run once a section reaches its size cap', () => {
    const nodes = Array.from({ length: 450 }, (_, i) => ['p', {}, `p${i}`] as ['p', {}, string])
    const sections = groupIntoSections(nodes)
    expect(sections.map((section) => (section as unknown[]).length - 2)).toEqual([200, 200, 50])
  })
})
