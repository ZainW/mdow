import { fireEvent, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { CommandPalette } from './CommandPalette'
import { useAppStore } from '../store/app-store'
import { renderWithProviders } from '../test/renderWithProviders'

const openMarkdownFile = vi.fn()

vi.mock('../hooks/useOpenMarkdownFile', () => ({
  useOpenMarkdownFile: () => openMarkdownFile,
}))

const recentsMock = vi.hoisted(() => ({ value: [] as string[] }))

vi.mock('../hooks/useRecents', () => ({
  useRecents: () => ({ data: recentsMock.value }),
}))

describe('CommandPalette', () => {
  beforeEach(() => {
    openMarkdownFile.mockReset()
    recentsMock.value = []
    useAppStore.setState({
      commandPaletteOpen: true,
      settingsOpen: false,
      sidebarOpen: true,
      wideMode: false,
      splitView: false,
      tabs: [],
      activeTabId: null,
      primaryPaneTabId: null,
      secondaryPaneTabId: null,
      folderTree: [
        {
          name: 'readme.md',
          path: '/docs/readme.md',
          isDirectory: false,
        },
      ],
    })
  })

  it('lists folder files and deduplicated recents', async () => {
    useAppStore.setState({
      commandPaletteOpen: true,
      folderTree: [],
    })
    recentsMock.value = ['/other/recent.md']
    renderWithProviders(<CommandPalette />)

    expect(await screen.findByText('recent.md')).toBeInTheDocument()
  })

  it('deduplicates recents that already appear in the folder tree', async () => {
    recentsMock.value = ['/docs/readme.md']
    renderWithProviders(<CommandPalette />)

    expect(await screen.findByText('readme.md')).toBeInTheDocument()
    expect(screen.getAllByText('readme.md')).toHaveLength(1)
  })

  it('opens a selected file and closes the palette', async () => {
    openMarkdownFile.mockResolvedValue(undefined)
    renderWithProviders(<CommandPalette />)

    fireEvent.click(screen.getByText('readme.md'))

    await waitFor(() => {
      expect(openMarkdownFile).toHaveBeenCalledWith('/docs/readme.md')
      expect(useAppStore.getState().commandPaletteOpen).toBe(false)
    })
  })

  it('shows the quick-open chrome when open', () => {
    renderWithProviders(<CommandPalette />)
    expect(screen.getByPlaceholderText('Search files and commands...')).toBeInTheDocument()
    expect(screen.getByText('run/open')).toBeInTheDocument()
    expect(screen.getByText('dismiss')).toBeInTheDocument()
  })

  it('opens settings from an action row and closes the palette', async () => {
    renderWithProviders(<CommandPalette />)

    fireEvent.click(screen.getByText('Settings'))

    await waitFor(() => {
      expect(useAppStore.getState().settingsOpen).toBe(true)
      expect(useAppStore.getState().commandPaletteOpen).toBe(false)
    })
  })

  it('toggles the sidebar from an action row', async () => {
    renderWithProviders(<CommandPalette />)

    fireEvent.click(screen.getByText('Toggle Sidebar'))

    await waitFor(() => {
      expect(useAppStore.getState().sidebarOpen).toBe(false)
      expect(useAppStore.getState().commandPaletteOpen).toBe(false)
    })
  })

  it('toggles wide mode from an action row', async () => {
    renderWithProviders(<CommandPalette />)

    fireEvent.click(screen.getByText('Toggle Wide Mode'))

    await waitFor(() => {
      expect(useAppStore.getState().wideMode).toBe(true)
      expect(useAppStore.getState().commandPaletteOpen).toBe(false)
    })
  })

  it('enables split view from an action row when multiple tabs are open', async () => {
    useAppStore.getState().openTab({ path: '/docs/one.md', content: 'one' })
    useAppStore.getState().openTab({ path: '/docs/two.md', content: 'two' })
    useAppStore.setState({ commandPaletteOpen: true })
    renderWithProviders(<CommandPalette />)

    fireEvent.click(screen.getByText('Toggle Split View'))

    await waitFor(() => {
      expect(useAppStore.getState().splitView).toBe(true)
      expect(useAppStore.getState().commandPaletteOpen).toBe(false)
    })
  })
})
