import { renderHook, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useMarkdownRender } from './useMarkdownRender'
import { useAppStore } from '../store/app-store'

const markdownMock = vi.hoisted(() => ({
  renderMarkdown: vi.fn(),
}))

vi.mock('../lib/markdown', () => ({
  renderMarkdown: markdownMock.renderMarkdown,
}))

describe('useMarkdownRender', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    useAppStore.setState({
      renderCache: new Map(),
      docHeadings: [],
      activeHeadingId: null,
    })
  })

  it('renders markdown and syncs headings to the app store', async () => {
    markdownMock.renderMarkdown.mockResolvedValue({
      tree: { nodes: [] },
      mermaidBlocks: [],
      headings: [{ level: 1, text: 'Title', id: 'title' }],
      frontmatter: {},
    })

    const { result } = renderHook(() =>
      useMarkdownRender({ tabId: 'tab-1', content: '# Title', retryKey: 0 }),
    )

    expect(result.current.isRendering).toBe(true)
    await waitFor(() => expect(result.current.renderResult?.headings[0]?.id).toBe('title'))
    expect(markdownMock.renderMarkdown).toHaveBeenCalledWith('# Title')
    expect(useAppStore.getState().docHeadings).toEqual([{ level: 1, text: 'Title', id: 'title' }])
    expect(useAppStore.getState().activeHeadingId).toBe('title')
  })
})
