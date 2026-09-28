import { render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { WelcomeView } from './WelcomeView'

vi.mock('../hooks/useOpenMarkdownFile', () => ({
  useOpenMarkdownFile: () => vi.fn(),
}))

const recentsMock = vi.hoisted(() => ({ value: [] as string[] }))

vi.mock('../hooks/useRecents', () => ({
  useRecents: () => ({ data: recentsMock.value }),
}))

function renderWithClient(ui: React.ReactElement) {
  const client = new QueryClient()
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>)
}

describe('WelcomeView', () => {
  beforeEach(() => {
    recentsMock.value = []
  })

  it('shows the tagline and both open actions with their shortcuts', () => {
    renderWithClient(<WelcomeView />)
    expect(screen.getByText('Open a Markdown file or folder to start reading.')).toBeVisible()
    // Tests run without a platform, so shortcuts use the Windows/Linux spelling.
    expect(screen.getByRole('button', { name: /Open File/ })).toHaveTextContent('Ctrl+O')
    expect(screen.getByRole('button', { name: /Open Folder/ })).toHaveTextContent('Ctrl+Shift+O')
  })

  it('lists recents with their parent folder so duplicates are distinguishable', () => {
    recentsMock.value = ['/Users/zain/mdow/README.md', '/Users/zain/flagship/README.md']
    renderWithClient(<WelcomeView />)
    const rows = screen.getAllByRole('button', { name: /README\.md/ })
    expect(rows).toHaveLength(2)
    expect(rows[0]).toHaveTextContent('mdow')
    expect(rows[1]).toHaveTextContent('flagship')
    expect(screen.getByText('Ctrl+K to search all')).toBeInTheDocument()
  })

  it('caps the recent list at five', () => {
    recentsMock.value = Array.from({ length: 8 }, (_, i) => `/docs/f${i}.md`)
    renderWithClient(<WelcomeView />)
    expect(screen.getAllByRole('button', { name: /f\d\.md/ })).toHaveLength(5)
  })

  it('hides the Recent section when there are no recents', () => {
    renderWithClient(<WelcomeView />)
    expect(screen.queryByText('Recent')).not.toBeInTheDocument()
  })

  it('shows a one-line drop hint', () => {
    renderWithClient(<WelcomeView />)
    expect(screen.getByText('Or drop files and folders anywhere in this window.')).toBeVisible()
  })

  it('shows a dev samples button in development', () => {
    renderWithClient(<WelcomeView />)
    expect(screen.getByRole('button', { name: /Dev samples/i })).toBeInTheDocument()
  })
})
