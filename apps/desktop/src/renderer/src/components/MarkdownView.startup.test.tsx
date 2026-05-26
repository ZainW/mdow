import { render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { MarkdownView } from './MarkdownView'
import type { Tab } from '../store/app-store'

const markdownMock = vi.hoisted(() => ({
  initMarkdown: vi.fn(),
  renderMarkdown: vi.fn(),
}))

vi.mock('../lib/markdown', () => ({
  initMarkdown: markdownMock.initMarkdown,
  renderMarkdown: markdownMock.renderMarkdown,
  getCachedMarkdownRender: vi.fn(() => undefined),
}))

vi.mock('../lib/mermaid', () => ({
  initMermaid: vi.fn(),
  renderMermaidBlock: vi.fn(),
  updateMermaidTheme: vi.fn(),
}))

vi.mock('../hooks/useDocumentSearch', () => ({
  useDocumentSearch: () => ({
    matchCount: 0,
    currentIndex: -1,
    next: vi.fn(),
    prev: vi.fn(),
    clear: vi.fn(),
  }),
}))

vi.mock('@comark/react', () => ({
  ComarkRenderer: ({ tree }: { tree: { nodes: unknown[] } }) => (
    <>
      {tree.nodes.map((node) =>
        Array.isArray(node) && node[0] === 'h1' && typeof node[2] === 'string' ? (
          <h1 key={node[2]}>{node[2]}</h1>
        ) : null,
      )}
    </>
  ),
}))

vi.mock('@comark/react/components/Math', () => ({
  Math: () => null,
}))

class MockIntersectionObserver {
  observe = vi.fn()
  disconnect = vi.fn()
}

const tab: Tab = {
  id: 'tab-1',
  path: '/tmp/startup.md',
  content: '# Startup',
  scrollPosition: 0,
}

beforeEach(() => {
  vi.clearAllMocks()
  Object.defineProperty(globalThis, 'IntersectionObserver', {
    value: MockIntersectionObserver,
    configurable: true,
  })
  markdownMock.initMarkdown.mockImplementation(
    () =>
      new Promise<void>(() => {
        // Keep initialization pending to prove first render is not gated on warm-up.
      }),
  )
  markdownMock.renderMarkdown.mockResolvedValue({
    tree: { nodes: [['h1', { id: 'startup' }, 'Startup']] },
    mermaidBlocks: [],
    headings: [{ level: 1, text: 'Startup', id: 'startup' }],
    frontmatter: {},
  })
})

describe('MarkdownView startup', () => {
  it('starts rendering the active tab before markdown warm-up resolves', async () => {
    render(<MarkdownView tab={tab} />)

    await waitFor(() => expect(markdownMock.renderMarkdown).toHaveBeenCalledWith('# Startup'))
    expect(screen.queryByText('Loading...')).not.toBeInTheDocument()
    expect(await screen.findByRole('heading', { name: 'Startup' })).toBeInTheDocument()
  })
})
