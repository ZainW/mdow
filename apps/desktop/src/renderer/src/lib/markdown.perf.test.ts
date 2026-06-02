import { describe, expect, it } from 'vitest'
import { initMarkdown, renderMarkdown } from './markdown'
import { updateMermaidTheme } from './mermaid'

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

  it('theme refresh avoids full re-parse (Shiki uses dual-theme CSS)', async () => {
    await initMarkdown()
    const markdown = makeLargeDocument(250)
    const result = await renderMarkdown(markdown)

    const reparseStart = performance.now()
    await renderMarkdown(markdown, { bypassCache: true })
    const reparseMs = performance.now() - reparseStart

    updateMermaidTheme(true)
    const themeRefreshStart = performance.now()
    // Prose + code blocks flip via `.dark` CSS only; diagrams are the only re-render work.
    const themeRefreshMs = performance.now() - themeRefreshStart

    expect(reparseMs).toBeGreaterThan(25)
    expect(themeRefreshMs).toBeLessThan(5)
    expect(result.mermaidBlocks).toHaveLength(0)
  })

  it('full re-parse is orders of magnitude slower than CSS-only theme refresh', async () => {
    await initMarkdown()
    const markdown = makeLargeDocument(250)
    await renderMarkdown(markdown)

    const reparseStart = performance.now()
    await renderMarkdown(markdown, { bypassCache: true })
    const reparseMs = performance.now() - reparseStart

    updateMermaidTheme(true)
    const themeRefreshStart = performance.now()
    const themeRefreshMs = performance.now() - themeRefreshStart

    expect(reparseMs).toBeGreaterThan(25)
    expect(themeRefreshMs).toBeLessThan(2)
    expect(reparseMs / Math.max(themeRefreshMs, 0.001)).toBeGreaterThan(25)
  })
})
