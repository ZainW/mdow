import { beforeEach, describe, expect, it, vi } from 'vitest'

const pluginMock = vi.hoisted(() => ({
  highlightFactory: vi.fn(() => ({ name: 'highlight' })),
  highlightCodeBlocks: vi.fn((tree: unknown) => Promise.resolve(tree)),
  mathFactory: vi.fn(() => ({ name: 'math' })),
  mermaidFactory: vi.fn(() => ({ name: 'mermaid' })),
}))

vi.mock('comark/plugins/highlight', () => ({
  default: pluginMock.highlightFactory,
  highlightCodeBlocks: pluginMock.highlightCodeBlocks,
}))

vi.mock('comark/plugins/math', () => ({
  default: pluginMock.mathFactory,
}))

vi.mock('comark/plugins/mermaid', () => ({
  default: pluginMock.mermaidFactory,
}))

describe('lazy markdown plugins', () => {
  beforeEach(() => {
    vi.resetModules()
    vi.clearAllMocks()
  })

  it('renders plain markdown without loading optional heavy plugins', async () => {
    const { renderMarkdown } = await import('./markdown')

    const result = await renderMarkdown('# Plain\n\nNo math, code, or diagrams.', {
      bypassCache: true,
    })

    expect(result.headings).toEqual([{ level: 1, text: 'Plain', id: 'plain' }])
    expect(pluginMock.highlightFactory).not.toHaveBeenCalled()
    expect(pluginMock.highlightCodeBlocks).not.toHaveBeenCalled()
    expect(pluginMock.mathFactory).not.toHaveBeenCalled()
    expect(pluginMock.mermaidFactory).not.toHaveBeenCalled()
  })
})
