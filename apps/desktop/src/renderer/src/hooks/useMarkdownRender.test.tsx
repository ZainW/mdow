import { renderHook, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useMarkdownRender } from './useMarkdownRender'
import { useAppStore } from '../store/app-store'

const markdownMock = vi.hoisted(() => ({
  renderMarkdown: vi.fn<(text: string) => Promise<unknown>>(),
}))

vi.mock('../lib/markdown-client', () => ({
  renderDocument: (text: string) => markdownMock.renderMarkdown(text),
}))

// Stands in for a multi-megabyte document; the real slicer has its own tests.
const HUGE = 'huge document'
const HEAD = 'huge opening'

vi.mock('../lib/markdown-preview', () => ({
  sliceDocumentHead: (text: string) => (text === HUGE ? HEAD : null),
}))

function renderWithHeading(id: string) {
  return {
    tree: { nodes: [] },
    mermaidBlocks: [],
    headings: [{ level: 2, text: id, id }],
    frontmatter: {},
  }
}

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

  it('shows the opening of a very large document until the full render lands', async () => {
    let finishFull: ((value: ReturnType<typeof renderWithHeading>) => void) | undefined
    markdownMock.renderMarkdown.mockImplementation((text: string) =>
      text === HUGE
        ? new Promise((resolve) => (finishFull = resolve))
        : Promise.resolve(renderWithHeading('preview')),
    )

    const { result: hook } = renderHook(() =>
      useMarkdownRender({ tabId: 'tab-1', content: HUGE, retryKey: 0, allowPreview: true }),
    )

    await waitFor(() => expect(hook.current.isPartial).toBe(true))
    expect(hook.current.renderResult?.headings[0]?.id).toBe('preview')
    expect(hook.current.isRendering).toBe(true)
    // The outline waits for the whole document.
    expect(useAppStore.getState().docHeadings).toEqual([])
    expect(useAppStore.getState().renderCache.has('tab-1')).toBe(false)

    finishFull?.(renderWithHeading('full'))
    await waitFor(() => expect(hook.current.isPartial).toBe(false))
    expect(hook.current.renderResult?.headings[0]?.id).toBe('full')
    expect(useAppStore.getState().docHeadings[0]?.id).toBe('full')
  })

  it('keeps the rendered document instead of previewing it again on reload', async () => {
    markdownMock.renderMarkdown.mockResolvedValue(renderWithHeading('full'))
    const { result: hook, rerender } = renderHook(
      ({ content }) =>
        useMarkdownRender({ tabId: 'tab-1', content, retryKey: 0, allowPreview: true }),
      { initialProps: { content: 'small document' } },
    )
    await waitFor(() => expect(hook.current.renderResult).not.toBeNull())
    useAppStore.getState().renderCache.delete('tab-1')

    rerender({ content: HUGE })
    await waitFor(() => expect(markdownMock.renderMarkdown).toHaveBeenCalledWith(HUGE))
    expect(markdownMock.renderMarkdown).not.toHaveBeenCalledWith(HEAD)
    expect(hook.current.isPartial).toBe(false)
  })

  it('previews a huge document opened in a pane that already shows another one', async () => {
    let finishFull: ((value: ReturnType<typeof renderWithHeading>) => void) | undefined
    markdownMock.renderMarkdown.mockImplementation((text: string) =>
      text === HUGE
        ? new Promise((resolve) => (finishFull = resolve))
        : Promise.resolve(renderWithHeading(text === HEAD ? 'preview' : 'small')),
    )
    const { result: hook, rerender } = renderHook(
      ({ tabId, content }) =>
        useMarkdownRender({ tabId, content, retryKey: 0, allowPreview: true }),
      { initialProps: { tabId: 'tab-1', content: 'small document' } },
    )
    await waitFor(() => expect(hook.current.renderResult?.headings[0]?.id).toBe('small'))

    rerender({ tabId: 'tab-2', content: HUGE })
    await waitFor(() => expect(hook.current.isPartial).toBe(true))
    expect(hook.current.renderResult?.headings[0]?.id).toBe('preview')
    finishFull?.(renderWithHeading('full'))
    await waitFor(() => expect(hook.current.isPartial).toBe(false))
  })

  it('skips the preview when restoring a saved position', async () => {
    markdownMock.renderMarkdown.mockResolvedValue({
      tree: { nodes: [] },
      mermaidBlocks: [],
      headings: [],
      frontmatter: {},
    })

    const { result: hook } = renderHook(() =>
      useMarkdownRender({ tabId: 'tab-1', content: HUGE, retryKey: 0 }),
    )

    await waitFor(() => expect(hook.current.renderResult).not.toBeNull())
    expect(markdownMock.renderMarkdown).toHaveBeenCalledTimes(1)
    expect(markdownMock.renderMarkdown).toHaveBeenCalledWith(HUGE)
  })
})
