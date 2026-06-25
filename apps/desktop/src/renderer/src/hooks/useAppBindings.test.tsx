import { fireEvent, render } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { describe, expect, it, beforeEach, vi } from 'vitest'
import type { ReactNode } from 'react'
import { useAppKeyboardShortcuts, useAppMenuBindings } from './useAppBindings'
import { useAppStore } from '../store/app-store'
import { createMinimalWindowApi, stubWindowApi } from '../test/stubWindowApi'

const openMarkdownFileMock = vi.hoisted(() => vi.fn())
const menuCallbacks = vi.hoisted(() => ({
  openRecent: null as ((path: string) => void) | null,
}))

vi.mock('../hooks/useOpenMarkdownFile', () => ({
  useOpenMarkdownFile: () => openMarkdownFileMock,
}))

stubWindowApi(() =>
  createMinimalWindowApi({
    openFileDialog: vi.fn().mockResolvedValue(null),
    openFolderDialog: vi.fn().mockResolvedValue(null),
    setActiveFileWatch: vi.fn().mockResolvedValue(undefined),
    closeWindow: vi.fn().mockResolvedValue(undefined),
    onMenuOpenFile: vi.fn(() => vi.fn()),
    onMenuOpenFolder: vi.fn(() => vi.fn()),
    onFileOpened: vi.fn(() => vi.fn()),
    onFileChanged: vi.fn(() => vi.fn()),
    onFileDeleted: vi.fn(() => vi.fn()),
    onMenuFind: vi.fn(() => vi.fn()),
    onMenuToggleSidebar: vi.fn(() => vi.fn()),
    onMenuZoomIn: vi.fn(() => vi.fn()),
    onMenuZoomOut: vi.fn(() => vi.fn()),
    onMenuZoomReset: vi.fn(() => vi.fn()),
    onMenuShortcuts: vi.fn(() => vi.fn()),
    onMenuSettings: vi.fn(() => vi.fn()),
    onMenuCloseTab: vi.fn(() => vi.fn()),
    onMenuOpenRecent: vi.fn((callback: (path: string) => void) => {
      menuCallbacks.openRecent = callback
      return vi.fn()
    }),
  }),
)

function ShortcutsHarness() {
  useAppKeyboardShortcuts()
  return null
}

function MenuBindingsHarness() {
  useAppMenuBindings()
  return null
}

function wrapper({ children }: { children: ReactNode }) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>
}

function renderShortcuts() {
  return render(<ShortcutsHarness />, { wrapper })
}

function renderMenuBindings() {
  return render(<MenuBindingsHarness />, { wrapper })
}

function key(modifiers: Partial<KeyboardEventInit>, key: string) {
  fireEvent.keyDown(window, { key, bubbles: true, ...modifiers })
}

describe('useAppKeyboardShortcuts', () => {
  beforeEach(() => {
    openMarkdownFileMock.mockReset()
    menuCallbacks.openRecent = null
    useAppStore.setState({
      commandPaletteOpen: false,
      searchOpen: false,
      sidebarOpen: true,
      shortcutsDialogOpen: false,
      settingsOpen: false,
      zoomLevel: 100,
      tabs: [],
      activeTabId: null,
    })
  })

  it('Cmd+K opens the command palette', () => {
    renderShortcuts()
    key({ metaKey: true }, 'k')
    expect(useAppStore.getState().commandPaletteOpen).toBe(true)
  })

  it('Cmd+B toggles the sidebar', () => {
    renderShortcuts()
    key({ metaKey: true }, 'b')
    expect(useAppStore.getState().sidebarOpen).toBe(false)
  })

  it('Cmd+F opens search', () => {
    renderShortcuts()
    key({ metaKey: true }, 'f')
    expect(useAppStore.getState().searchOpen).toBe(true)
  })

  it('Cmd+= zooms in', () => {
    renderShortcuts()
    key({ metaKey: true }, '=')
    expect(useAppStore.getState().zoomLevel).toBe(110)
  })

  it('Cmd+- zooms out', () => {
    renderShortcuts()
    key({ metaKey: true }, '-')
    expect(useAppStore.getState().zoomLevel).toBe(90)
  })

  it('Cmd+0 resets zoom', () => {
    useAppStore.setState({ zoomLevel: 130 })
    renderShortcuts()
    key({ metaKey: true }, '0')
    expect(useAppStore.getState().zoomLevel).toBe(100)
  })

  it('Cmd+/ opens shortcuts dialog', () => {
    renderShortcuts()
    key({ metaKey: true }, '/')
    expect(useAppStore.getState().shortcutsDialogOpen).toBe(true)
  })

  it('Cmd+, opens settings', () => {
    renderShortcuts()
    key({ metaKey: true }, ',')
    expect(useAppStore.getState().settingsOpen).toBe(true)
  })

  it('Cmd+Alt+ArrowRight cycles tabs forward', () => {
    useAppStore.getState().openTab({ path: '/a.md', content: 'a' })
    useAppStore.getState().openTab({ path: '/b.md', content: 'b' })
    const firstId = useAppStore.getState().tabs[0].id
    useAppStore.getState().setActiveTab(firstId)

    renderShortcuts()
    key({ metaKey: true, altKey: true }, 'ArrowRight')

    expect(useAppStore.getState().activeTabId).toBe(useAppStore.getState().tabs[1].id)
  })

  it('Cmd+1 selects the first tab', () => {
    useAppStore.getState().openTab({ path: '/a.md', content: 'a' })
    useAppStore.getState().openTab({ path: '/b.md', content: 'b' })
    useAppStore.getState().setActiveTab(useAppStore.getState().tabs[1].id)

    renderShortcuts()
    key({ metaKey: true }, '1')

    expect(useAppStore.getState().activeTabId).toBe(useAppStore.getState().tabs[0].id)
  })
})

describe('useAppMenuBindings', () => {
  beforeEach(() => {
    openMarkdownFileMock.mockReset()
    menuCallbacks.openRecent = null
    useAppStore.setState({
      tabs: [],
      activeTabId: null,
      splitView: false,
      primaryPaneTabId: null,
      secondaryPaneTabId: null,
      openingPath: null,
    })
  })

  it('opens a recent file from the application menu', () => {
    renderMenuBindings()

    expect(menuCallbacks.openRecent).toBeTypeOf('function')
    menuCallbacks.openRecent?.('/docs/recent.md')

    expect(openMarkdownFileMock).toHaveBeenCalledWith('/docs/recent.md')
  })
})
