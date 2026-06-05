import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { ReactNode } from 'react'
import App from '../../App'
import type { CompanionProviderStatus } from '../../../../shared/types'
import { CompanionPanel } from './CompanionPanel'
import { ShortcutsDialog } from '../ShortcutsDialog'
import { SettingsDialog } from '../SettingsDialog'
import { useAppKeyboardShortcuts } from '../../hooks/useAppBindings'
import { useAppStore } from '../../store/app-store'

vi.mock('../MarkdownView', () => ({
  MarkdownView: () => <div>Markdown view</div>,
}))

const saveSettings = vi.hoisted(() => vi.fn(() => Promise.resolve()))

const availableProvider: CompanionProviderStatus = {
  id: 'opencode',
  label: 'opencode',
  command: 'opencode acp',
  status: 'available',
}

function ShortcutsHarness() {
  useAppKeyboardShortcuts()
  return null
}

function wrapper({ children }: { children: ReactNode }) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>
}

beforeEach(() => {
  saveSettings.mockClear()
  Object.defineProperty(window, 'api', {
    configurable: true,
    value: {
      saveCompanionSettings: saveSettings,
      setTheme: vi.fn(),
      saveAppState: vi.fn(),
      setAutoUpdateScheduling: vi.fn(),
      getAppState: vi.fn(() =>
        Promise.resolve({
          zoomLevel: 100,
          lastFolder: null,
          windowBounds: null,
          sessionTabs: [],
          sessionActiveTabPath: null,
          contentFont: 'inter',
          codeFont: 'geist-mono',
          theme: 'system',
          autoUpdateEnabled: true,
          wideMode: false,
          interfaceScale: 'compact',
          readingWidth: 'standard',
          sidebarMode: 'recents',
          companionProvider: 'auto',
          companionCustomCommand: '',
        }),
      ),
      setWindowTitle: vi.fn(),
      setActiveFileWatch: vi.fn(),
      onThemeChanged: vi.fn(() => () => {}),
      onFolderChanged: vi.fn(() => () => {}),
      onMenuOpenFile: vi.fn(() => () => {}),
      onMenuOpenFolder: vi.fn(() => () => {}),
      onFileOpened: vi.fn(() => () => {}),
      onFileChanged: vi.fn(() => () => {}),
      onFileDeleted: vi.fn(() => () => {}),
      onMenuFind: vi.fn(() => () => {}),
      onMenuToggleSidebar: vi.fn(() => () => {}),
      onMenuZoomIn: vi.fn(() => () => {}),
      onMenuZoomOut: vi.fn(() => () => {}),
      onMenuZoomReset: vi.fn(() => () => {}),
      onMenuShortcuts: vi.fn(() => () => {}),
      onMenuSettings: vi.fn(() => () => {}),
      onMenuCloseTab: vi.fn(() => () => {}),
      onMenuCheckForUpdates: vi.fn(() => () => {}),
      onUpdateAvailable: vi.fn(() => () => {}),
      onUpdateDownloadProgress: vi.fn(() => () => {}),
      onUpdateDownloaded: vi.fn(() => () => {}),
      onUpdateUpToDate: vi.fn(() => () => {}),
      onUpdateError: vi.fn(() => () => {}),
      checkForUpdates: vi.fn(() => Promise.resolve()),
      downloadUpdate: vi.fn(() => Promise.resolve()),
      installUpdate: vi.fn(() => Promise.resolve()),
      detectCompanionProviders: vi.fn(() => Promise.resolve([])),
      getCompanionSettings: vi.fn(() => Promise.resolve({ provider: 'auto', customCommand: '' })),
      sendCompanionMessage: vi.fn(() => Promise.resolve()),
      cancelCompanionMessage: vi.fn(() => Promise.resolve()),
      onCompanionUpdate: vi.fn(() => () => {}),
    },
  })
  useAppStore.setState({
    companionOpen: false,
    companionFullscreen: false,
    companionProvider: 'auto',
    companionCustomCommand: '',
    companionProviders: [],
    commandPaletteOpen: false,
    initialized: true,
  })
})

describe('companion settings integration', () => {
  it('renders companion provider settings', () => {
    render(<SettingsDialog open onOpenChange={() => {}} />)

    expect(screen.getByText('Companion')).toBeInTheDocument()
    expect(screen.getByRole('group', { name: 'Companion provider' })).toBeInTheDocument()
    expect(screen.getByLabelText('Custom ACP command')).toBeInTheDocument()
  })

  it('keeps settings reachable on short viewports', () => {
    render(<SettingsDialog open onOpenChange={() => {}} />)

    expect(screen.getByRole('dialog', { name: 'Settings' })).toHaveClass(
      'max-h-[calc(100vh-2rem)]',
      'overflow-y-auto',
    )
  })

  it('updates custom companion command while typing without saving until blur', async () => {
    useAppStore.setState({ companionOpen: true })
    render(<SettingsDialog open onOpenChange={() => {}} />)

    const input = screen.getByLabelText('Custom ACP command')
    fireEvent.change(input, {
      target: { value: 'custom acp' },
    })

    expect(useAppStore.getState().companionCustomCommand).toBe('custom acp')
    expect(saveSettings).not.toHaveBeenCalled()
    expect(window.api.detectCompanionProviders).not.toHaveBeenCalled()

    fireEvent.blur(input)

    expect(saveSettings).toHaveBeenCalledWith({ provider: 'auto', customCommand: 'custom acp' })
    await waitFor(() => expect(window.api.detectCompanionProviders).toHaveBeenCalledTimes(1))
  })

  it('saves custom companion command on Enter without detecting providers while companion is closed', async () => {
    render(<SettingsDialog open onOpenChange={() => {}} />)

    const input = screen.getByLabelText('Custom ACP command')
    fireEvent.change(input, {
      target: { value: 'custom acp' },
    })

    expect(saveSettings).not.toHaveBeenCalled()
    fireEvent.keyDown(input, { key: 'Enter' })

    await waitFor(() =>
      expect(saveSettings).toHaveBeenCalledWith({
        provider: 'auto',
        customCommand: 'custom acp',
      }),
    )
    expect(window.api.detectCompanionProviders).not.toHaveBeenCalled()
  })

  it('persists the selected companion provider from settings', () => {
    render(<SettingsDialog open onOpenChange={() => {}} />)

    fireEvent.click(screen.getByRole('button', { name: 'Codex' }))

    expect(useAppStore.getState().companionProvider).toBe('codex')
    expect(saveSettings).toHaveBeenCalledWith({ provider: 'codex', customCommand: '' })
  })

  it('refreshes open companion providers after settings change', async () => {
    vi.mocked(window.api.detectCompanionProviders)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([availableProvider])
    useAppStore.setState({ companionOpen: true, companionProviders: [] })

    const { rerender } = render(
      <>
        <CompanionPanel />
        <SettingsDialog open onOpenChange={() => {}} />
      </>,
    )

    await waitFor(() => expect(window.api.detectCompanionProviders).toHaveBeenCalledTimes(1))
    expect(screen.getByText('Connect a local companion')).toBeInTheDocument()

    const input = screen.getByLabelText('Custom ACP command')
    fireEvent.change(input, {
      target: { value: 'opencode acp' },
    })

    expect(saveSettings).not.toHaveBeenCalled()
    fireEvent.blur(input)

    expect(saveSettings).toHaveBeenCalledWith({ provider: 'auto', customCommand: 'opencode acp' })
    await waitFor(() => expect(window.api.detectCompanionProviders).toHaveBeenCalledTimes(2))
    expect(useAppStore.getState().companionProviders).toEqual([availableProvider])
    rerender(
      <>
        <CompanionPanel />
        <SettingsDialog open={false} onOpenChange={() => {}} />
      </>,
    )
    expect(screen.getByRole('textbox', { name: 'Companion prompt' })).toBeInTheDocument()
  })

  it('ignores stale companion provider detections after newer settings commits', async () => {
    let resolveFirst!: (providers: (typeof availableProvider)[]) => void
    let resolveSecond!: (providers: (typeof availableProvider)[]) => void
    const staleProvider = { ...availableProvider, command: 'stale acp' }
    vi.mocked(window.api.detectCompanionProviders)
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveFirst = resolve
          }),
      )
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveSecond = resolve
          }),
      )
    useAppStore.setState({ companionOpen: true, companionProviders: [] })
    render(<SettingsDialog open onOpenChange={() => {}} />)

    fireEvent.click(screen.getByRole('button', { name: 'Codex' }))
    await waitFor(() => expect(window.api.detectCompanionProviders).toHaveBeenCalledTimes(1))
    fireEvent.click(screen.getByRole('button', { name: 'opencode' }))

    await waitFor(() => expect(window.api.detectCompanionProviders).toHaveBeenCalledTimes(2))
    resolveSecond([availableProvider])
    await waitFor(() =>
      expect(useAppStore.getState().companionProviders).toEqual([availableProvider]),
    )
    resolveFirst([staleProvider])

    await new Promise((resolve) => setTimeout(resolve, 0))
    expect(useAppStore.getState().companionProviders).toEqual([availableProvider])
  })

  it('surfaces companion settings save failures without leaving an unhandled rejection', async () => {
    saveSettings.mockRejectedValueOnce(new Error('Settings save failed'))
    render(<SettingsDialog open onOpenChange={() => {}} />)

    const input = screen.getByLabelText('Custom ACP command')
    fireEvent.change(input, {
      target: { value: 'custom acp' },
    })
    fireEvent.blur(input)

    await waitFor(() => {
      expect(useAppStore.getState().companionError).toBe('Settings save failed')
    })
    expect(screen.getByRole('alert')).toHaveTextContent('Settings save failed')
    expect(window.api.detectCompanionProviders).not.toHaveBeenCalled()
  })

  it('resets companion settings to defaults and persists them', () => {
    useAppStore.setState({ companionProvider: 'codex', companionCustomCommand: 'custom acp' })
    render(<SettingsDialog open onOpenChange={() => {}} />)

    fireEvent.click(screen.getByRole('button', { name: 'Reset to defaults' }))

    expect(useAppStore.getState().companionProvider).toBe('auto')
    expect(useAppStore.getState().companionCustomCommand).toBe('')
    expect(screen.getByRole('button', { name: 'Auto' })).toHaveAttribute('aria-pressed', 'true')
    expect(screen.getByLabelText('Custom ACP command')).toHaveValue('')
    expect(saveSettings).toHaveBeenCalledWith({ provider: 'auto', customCommand: '' })
  })

  it('Cmd+Shift+K toggles the companion panel without opening the command palette', () => {
    render(<ShortcutsHarness />)

    fireEvent.keyDown(window, { key: 'k', metaKey: true, shiftKey: true, bubbles: true })

    expect(useAppStore.getState().companionOpen).toBe(true)
    expect(useAppStore.getState().commandPaletteOpen).toBe(false)

    fireEvent.keyDown(window, { key: 'k', metaKey: true, shiftKey: true, bubbles: true })

    expect(useAppStore.getState().companionOpen).toBe(false)
  })

  it('mounts companion surfaces in the app shell', () => {
    useAppStore.setState({ companionOpen: true, companionFullscreen: false })

    const { unmount } = render(<App />, { wrapper })

    expect(screen.getByRole('complementary', { name: 'AI companion' })).toBeInTheDocument()
    expect(screen.getByRole('main', { name: 'Document' }).parentElement).toHaveClass('relative')

    unmount()
    useAppStore.setState({ companionOpen: false, companionFullscreen: true })
    render(<App />, { wrapper })

    expect(screen.getByRole('dialog', { name: 'AI companion' })).toBeInTheDocument()
  })

  it('does not start companion provider detection until the panel opens', async () => {
    render(<App />, { wrapper })

    await waitFor(() => expect(window.api.setWindowTitle).toHaveBeenCalled())
    expect(window.api.getCompanionSettings).not.toHaveBeenCalled()
    expect(window.api.detectCompanionProviders).not.toHaveBeenCalled()
    expect(window.api.onCompanionUpdate).not.toHaveBeenCalled()

    fireEvent.keyDown(window, { key: 'k', metaKey: true, shiftKey: true, bubbles: true })

    expect(screen.getByRole('complementary', { name: 'AI companion' })).toBeInTheDocument()
    await waitFor(() => expect(window.api.getCompanionSettings).toHaveBeenCalledTimes(1))
    expect(window.api.detectCompanionProviders).toHaveBeenCalledTimes(1)
    expect(window.api.onCompanionUpdate).toHaveBeenCalledTimes(1)
  })

  it('lists the companion shortcut in keyboard shortcuts', () => {
    render(<ShortcutsDialog open onOpenChange={() => {}} />)

    expect(screen.getByText('AI companion')).toBeInTheDocument()
    expect(screen.getByText(/⇧ K/)).toBeInTheDocument()
  })
})
