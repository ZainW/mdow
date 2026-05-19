import { render, screen, fireEvent } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { Sidebar } from './Sidebar'
import { SidebarProvider } from './ui/sidebar'
import { useAppStore } from '../store/app-store'

const recentsMock = vi.hoisted(() => ({ value: [] as string[] }))

vi.mock('../hooks/useRecents', () => ({
  useRecents: () => ({ data: recentsMock.value }),
}))

vi.mock('../hooks/useFolderTree', () => ({
  useFolderTree: () => {},
}))

vi.mock('../hooks/useOpenMarkdownFile', () => ({
  useOpenMarkdownFile: () => vi.fn(),
}))

function renderSidebar() {
  const client = new QueryClient()
  return render(
    <QueryClientProvider client={client}>
      <SidebarProvider>
        <Sidebar />
      </SidebarProvider>
    </QueryClientProvider>,
  )
}

describe('Sidebar', () => {
  beforeEach(() => {
    recentsMock.value = []
    useAppStore.setState({
      tabs: [],
      activeTabId: null,
      openFolderPath: null,
      folderTree: [],
      docHeadings: [],
    })
  })

  it('exposes a Sidebar mode radiogroup with three options', () => {
    renderSidebar()
    const group = screen.getByRole('radiogroup', { name: 'Sidebar mode' })
    expect(group).toBeInTheDocument()
    const options = screen.getAllByRole('radio')
    expect(options).toHaveLength(3)
    const labels = options.map((o) => o.getAttribute('aria-label'))
    expect(labels).toEqual(['Recents', 'Folder', 'Outline'])
  })

  it('marks the active mode as aria-checked', () => {
    renderSidebar()
    const folder = screen.getByRole('radio', { name: 'Folder' })
    expect(folder.getAttribute('aria-checked')).toBe('false')
    fireEvent.click(folder)
    expect(folder.getAttribute('aria-checked')).toBe('true')
  })

  it('shows the empty-state when in Folder mode with no folder open', () => {
    renderSidebar()
    fireEvent.click(screen.getByRole('radio', { name: 'Folder' }))
    expect(screen.getByText('No folder open')).toBeInTheDocument()
  })

  it('shows the recents empty-state when no files have been opened', () => {
    renderSidebar()
    expect(screen.getByText('No recents yet')).toBeInTheDocument()
  })

  it('ArrowDown rotates focus between sidebar mode options', () => {
    renderSidebar()
    const recents = screen.getByRole('radio', { name: 'Recents' })
    const folder = screen.getByRole('radio', { name: 'Folder' })
    recents.focus()
    fireEvent.keyDown(recents, { key: 'ArrowDown' })
    expect(document.activeElement).toBe(folder)
  })

  it('marks only the active sidebar mode as tabIndex=0', () => {
    renderSidebar()
    const radios = screen.getAllByRole('radio')
    // Initial mode is recents → it's tabIndex=0, others are -1
    expect(radios[0].getAttribute('tabindex')).toBe('0')
    expect(radios[1].getAttribute('tabindex')).toBe('-1')
    expect(radios[2].getAttribute('tabindex')).toBe('-1')
  })
})
