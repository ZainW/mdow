import { describe, expect, it } from 'vitest'
import { initMarkdown, renderMarkdown } from './markdown'

function makeLargeDocument(sectionCount: number): string {
  return Array.from({ length: sectionCount }, (_, i) =>
    [
      `## Section ${i}`,
      '',
      `This is a paragraph with enough text to exercise inline parsing for section ${i}.`,
      '',
      '- first item',
      '- second item with **strong text** and `inline code`',
      '- third item with [a link](https://example.com)',
      '',
      '```ts',
      `const value${i} = ${i}`,
      `console.log(value${i})`,
      '```',
      '',
    ].join('\n'),
  ).join('\n')
}

describe('markdown rendering performance', () => {
  it('initializes the parser within the warm-start budget', async () => {
    const startedAt = performance.now()

    await initMarkdown()

    expect(performance.now() - startedAt).toBeLessThan(2_500)
  })

  it('renders a large mixed markdown document within the interaction budget', async () => {
    await initMarkdown()
    const markdown = makeLargeDocument(180)
    const startedAt = performance.now()

    const result = await renderMarkdown(markdown)

    expect(performance.now() - startedAt).toBeLessThan(1_500)
    expect(result.headings).toHaveLength(180)
    expect(result.tree.nodes.length).toBeGreaterThan(0)
  })

  it('collects many mermaid blocks without dominating render time', async () => {
    await initMarkdown()
    const markdown = Array.from({ length: 60 }, (_, i) =>
      ['```mermaid', 'flowchart TD', `  A${i} --> B${i}`, '```'].join('\n'),
    ).join('\n\n')
    const startedAt = performance.now()

    const result = await renderMarkdown(markdown)

    expect(performance.now() - startedAt).toBeLessThan(750)
    expect(result.mermaidBlocks).toHaveLength(60)
  })
})
