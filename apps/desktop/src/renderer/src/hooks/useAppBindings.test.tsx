import { fireEvent, render } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { describe, expect, it, beforeEach } from 'vitest'
import type { ReactNode } from 'react'
import { useAppKeyboardShortcuts } from './useAppBindings'
import { useAppStore } from '../store/app-store'
import { createMinimalWindowApi, stubWindowApi } from '../test/stubWindowApi'

stubWindowApi(() => createMinimalWindowApi())

function ShortcutsHarness() {
  useAppKeyboardShortcuts()
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

function key(modifiers: Partial<KeyboardEventInit>, key: string) {
  fireEvent.keyDown(window, { key, bubbles: true, ...modifiers })
}

describe('useAppKeyboardShortcuts', () => {
  beforeEach(() => {
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
