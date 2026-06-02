import { render, screen, fireEvent } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { Sidebar } from './Sidebar'
import { SidebarProvider } from './ui/sidebar'
import { useAppStore } from '../store/app-store'

const recentsMock = vi.hoisted(() => ({ value: [] as string[] }))
const folderTreeMock = vi.hoisted(() => ({
  loaded: vi.fn(),
  rendered: vi.fn(),
}))

vi.mock('../hooks/useRecents', () => ({
  useRecents: () => ({ data: recentsMock.value }),
}))

vi.mock('../hooks/useFolderTree', () => ({
  useFolderTree: () => {},
}))

vi.mock('../hooks/useOpenMarkdownFile', () => ({
  useOpenMarkdownFile: () => vi.fn(),
}))

vi.mock('./FolderTree', () => {
  folderTreeMock.loaded()
  return {
    FolderTree: () => {
      folderTreeMock.rendered()
      return <div>Folder tree loaded</div>
    },
  }
})

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
      sidebarMode: 'recents',
    })
  })

  it('exposes Sidebar mode tabs inside a single sidebar surface', () => {
    renderSidebar()
    const sidebar = screen.getByRole('complementary', { name: 'Sidebar' })
    const group = screen.getByRole('radiogroup', { name: 'Sidebar mode' })

    expect(sidebar).toContainElement(group)
    expect(screen.queryByLabelText('Workspace actions')).not.toBeInTheDocument()

    const options = screen.getAllByRole('radio')
    expect(options).toHaveLength(3)
    expect(options.map((o) => o.textContent)).toEqual(['Recents', 'Folder', 'Outline'])
    expect(options.map((o) => o.getAttribute('aria-label'))).toEqual([
      'Recents',
      'Folder',
      'Outline',
    ])
  })

  it('marks the active mode as aria-checked', () => {
    renderSidebar()
    const folder = screen.getByRole('radio', { name: 'Folder' })
    expect(folder.getAttribute('aria-checked')).toBe('false')
    fireEvent.click(folder)
    expect(folder.getAttribute('aria-checked')).toBe('true')
  })

  it('does not render the old permanent sidebar rail actions', () => {
    renderSidebar()

    expect(screen.queryByRole('button', { name: 'Quick Open' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Open File' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Settings' })).not.toBeInTheDocument()
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

  it('does not load the folder tree module while another sidebar mode is active', () => {
    renderSidebar()
    expect(folderTreeMock.loaded).not.toHaveBeenCalled()
    expect(folderTreeMock.rendered).not.toHaveBeenCalled()
  })

  it('ArrowRight rotates focus between sidebar mode options', () => {
    renderSidebar()
    const recents = screen.getByRole('radio', { name: 'Recents' })
    const folder = screen.getByRole('radio', { name: 'Folder' })
    recents.focus()
    fireEvent.keyDown(recents, { key: 'ArrowRight' })
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
