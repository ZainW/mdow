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

  it('centers the hero when there are no recents', () => {
    const { container } = renderWithClient(<WelcomeView />)
    const heroColumn = container.querySelector('h2')?.parentElement
    expect(heroColumn?.className).toContain('items-center')
    expect(heroColumn?.className).toContain('text-center')
  })

  it('left-aligns the hero next to a recents column', () => {
    recentsMock.value = ['/Users/zain/a.md', '/Users/zain/b.md']
    const { container } = renderWithClient(<WelcomeView />)
    const heroColumn = container.querySelector('h2')?.parentElement
    expect(heroColumn?.className).not.toContain('items-center')
    expect(heroColumn?.className).not.toContain('text-center')
  })

  it('shows the rephrased drop hint', () => {
    renderWithClient(<WelcomeView />)
    expect(screen.getByText(/Anywhere in this window/)).toBeInTheDocument()
  })

  it('shows a dev samples button in development', () => {
    renderWithClient(<WelcomeView />)
    expect(screen.getByRole('button', { name: /Dev samples/i })).toBeInTheDocument()
  })
})
