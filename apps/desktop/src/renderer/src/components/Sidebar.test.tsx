import { render, screen, fireEvent } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { Sidebar } from './Sidebar'
import { SidebarProvider } from './ui/sidebar'
import { useAppStore } from '../store/app-store'
import { stubWindowApi } from '../test/stubWindowApi'

const recentsMock = vi.hoisted(() => ({ value: [] as string[] }))
const folderTreeMock = vi.hoisted(() => ({
  loaded: vi.fn(),
  rendered: vi.fn(),
}))

vi.mock('../hooks/useRecents', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../hooks/useRecents')>()),
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

  it('does not render the old permanent sidebar rail file actions', () => {
    renderSidebar()

    expect(screen.queryByRole('button', { name: 'Quick Open' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Open File' })).not.toBeInTheDocument()
  })

  it('opens settings from the sidebar footer', () => {
    useAppStore.setState({ settingsOpen: false })
    renderSidebar()

    fireEvent.click(screen.getByRole('button', { name: 'Settings' }))

    expect(useAppStore.getState().settingsOpen).toBe(true)
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

  it('keeps a section header row in every mode so the list never jumps', () => {
    renderSidebar()
    expect(screen.getByTestId('sidebar-section-header')).toHaveTextContent('Recent files')
    fireEvent.click(screen.getByRole('radio', { name: 'Folder' }))
    expect(screen.getByTestId('sidebar-section-header')).toHaveTextContent('No folder')
    expect(screen.getByRole('button', { name: 'Open folder' })).toBeInTheDocument()
    fireEvent.click(screen.getByRole('radio', { name: 'Outline' }))
    expect(screen.getByTestId('sidebar-section-header')).toHaveTextContent('No document')
  })

  it('shows the folder name and file count in Folder mode', () => {
    useAppStore.setState({
      sidebarMode: 'folder',
      openFolderPath: '/Users/zain/docs',
      folderTreeTruncated: false,
      folderTree: [
        {
          name: 'guides',
          path: '/Users/zain/docs/guides',
          isDirectory: true,
          children: [
            { name: 'a.md', path: '/Users/zain/docs/guides/a.md', isDirectory: false },
            { name: 'b.md', path: '/Users/zain/docs/guides/b.md', isDirectory: false },
          ],
        },
        { name: 'README.md', path: '/Users/zain/docs/README.md', isDirectory: false },
      ],
    })
    renderSidebar()
    const header = screen.getByTestId('sidebar-section-header')
    expect(header).toHaveTextContent('docs')
    expect(header).toHaveTextContent('3 files')
  })

  it('shows the document title and heading count in Outline mode', () => {
    useAppStore.setState({
      sidebarMode: 'outline',
      tabs: [{ id: 't1', path: '/a/guide.md', content: '', scrollPosition: 0 }],
      activeTabId: 't1',
      docHeadings: [
        { level: 1, text: 'Reading guide', id: 'reading-guide' },
        { level: 2, text: 'Opening', id: 'opening' },
      ],
    })
    renderSidebar()
    const header = screen.getByTestId('sidebar-section-header')
    expect(header).toHaveTextContent('Reading guide')
    expect(header).toHaveTextContent('2 headings')
  })

  it('shows the Settings shortcut hint in the footer', () => {
    renderSidebar()
    expect(screen.getByRole('button', { name: 'Settings' })).toHaveTextContent('Ctrl+,')
  })

  describe('recents', () => {
    const saveAppState = vi.fn().mockResolvedValue(undefined)
    stubWindowApi(() => ({ saveAppState, getRecents: vi.fn().mockResolvedValue([]) }))
    beforeEach(() => saveAppState.mockClear())

    it('renders single-line rows with the parent folder', () => {
      recentsMock.value = ['/Users/zain/mdow/README.md', '/Users/zain/flagship/README.md']
      renderSidebar()
      const rows = screen.getAllByTitle(/README\.md$/)
      expect(rows).toHaveLength(2)
      expect(rows[0]).toHaveTextContent('README.mdmdow')
      expect(rows[1]).toHaveTextContent('README.mdflagship')
    })

    it('clears recents from the section header', () => {
      recentsMock.value = ['/Users/zain/mdow/README.md']
      renderSidebar()
      fireEvent.click(screen.getByRole('button', { name: 'Clear recent files' }))
      expect(saveAppState).toHaveBeenCalledWith({ recents: [] })
    })

    it('hides Clear when there is nothing to clear', () => {
      renderSidebar()
      expect(screen.queryByRole('button', { name: 'Clear recent files' })).not.toBeInTheDocument()
    })
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
