import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { CompanionProviderStatus, CompanionUpdate } from '../../../../shared/types'
import { useAppStore } from '../../store/app-store'
import { CompanionFullscreen } from './CompanionFullscreen'
import { CompanionPanel } from './CompanionPanel'

const availableProvider: CompanionProviderStatus = {
  id: 'opencode',
  label: 'opencode',
  command: 'opencode acp',
  status: 'available',
}

const availableCustomProvider: CompanionProviderStatus = {
  id: 'custom',
  label: 'Custom ACP',
  command: 'custom acp',
  status: 'available',
}

const api = vi.hoisted(() => ({
  detectCompanionProviders: vi.fn((): Promise<CompanionProviderStatus[]> => Promise.resolve([])),
  getCompanionSettings: vi.fn(() => Promise.resolve({ provider: 'auto', customCommand: '' })),
  saveAppState: vi.fn(() => Promise.resolve()),
  sendCompanionMessage: vi.fn(() => Promise.resolve()),
  cancelCompanionMessage: vi.fn(() => Promise.resolve()),
  onCompanionUpdate: vi.fn((_callback: (update: CompanionUpdate) => void) => () => {}),
}))

describe('Companion UI', () => {
  beforeEach(() => {
    vi.resetAllMocks()
    api.detectCompanionProviders.mockResolvedValue([])
    api.getCompanionSettings.mockResolvedValue({ provider: 'auto', customCommand: '' })
    api.saveAppState.mockResolvedValue(undefined)
    api.sendCompanionMessage.mockResolvedValue(undefined)
    api.cancelCompanionMessage.mockResolvedValue(undefined)
    api.onCompanionUpdate.mockReturnValue(() => {})
    Object.defineProperty(window, 'api', { value: api, configurable: true })
    useAppStore.getState().resetCompanion()
    useAppStore.setState({ sidebarMode: 'folder' })
  })

  it('renders the right companion panel without changing left sidebar mode', () => {
    useAppStore.setState({ companionOpen: true })
    render(<CompanionPanel />)

    expect(screen.getByRole('complementary', { name: 'AI companion' })).toBeInTheDocument()
    expect(useAppStore.getState().sidebarMode).toBe('folder')
  })

  it('overlays the companion panel below desktop widths', () => {
    useAppStore.setState({ companionOpen: true })
    render(<CompanionPanel />)

    expect(screen.getByRole('complementary', { name: 'AI companion' })).toHaveClass(
      'fixed',
      'right-0',
      'lg:static',
      'lg:w-80',
    )
  })

  it('shows provider setup when no provider is available', () => {
    useAppStore.setState({ companionOpen: true, companionProviders: [] })
    render(<CompanionPanel />)

    expect(screen.getByText('Connect a local companion')).toBeInTheDocument()
    expect(screen.getByText('opencode acp')).toBeInTheDocument()
    expect(screen.getByText('npx --no-install @zed-industries/codex-acp')).toBeInTheDocument()
  })

  it('detects providers when setup renders without a composer', async () => {
    useAppStore.setState({ companionOpen: true, companionProviders: [] })
    render(<CompanionPanel />)

    expect(screen.queryByRole('textbox', { name: 'Companion prompt' })).toBeNull()
    await waitFor(() => expect(api.getCompanionSettings).toHaveBeenCalledTimes(1))
    expect(api.detectCompanionProviders).toHaveBeenCalledTimes(1)
  })

  it('shows provider setup when no provider is available even with existing messages', () => {
    useAppStore.setState({ companionOpen: true, companionProviders: [] })
    useAppStore.getState().appendCompanionMessage('user', 'Hello')
    render(<CompanionPanel />)

    expect(screen.getByText('Connect a local companion')).toBeInTheDocument()
    expect(screen.getByText('opencode acp')).toBeInTheDocument()
    expect(screen.getByText('npx --no-install @zed-industries/codex-acp')).toBeInTheDocument()
  })

  it('requires the selected custom provider to have a command and available detection result', () => {
    useAppStore.setState({
      companionOpen: true,
      companionProvider: 'custom',
      companionCustomCommand: '',
      companionProviders: [availableProvider],
    })
    render(<CompanionPanel />)

    expect(screen.getByText('Connect a local companion')).toBeInTheDocument()
    expect(screen.queryByRole('textbox', { name: 'Companion prompt' })).toBeNull()
    expect(screen.getByText('Not connected')).toBeInTheDocument()
  })

  it('uses the selected custom provider when a command and provider are available', () => {
    useAppStore.setState({
      companionOpen: true,
      companionProvider: 'custom',
      companionCustomCommand: 'custom acp',
      companionProviders: [availableProvider, availableCustomProvider],
    })
    render(<CompanionPanel />)

    expect(screen.getByRole('textbox', { name: 'Companion prompt' })).toBeInTheDocument()
    expect(screen.getByText('Custom ACP')).toBeInTheDocument()
  })

  it('expands the same conversation', () => {
    useAppStore.setState({ companionOpen: true, companionProviders: [availableProvider] })
    useAppStore.getState().appendCompanionMessage('user', 'Hello')
    render(
      <>
        <CompanionPanel />
        <CompanionFullscreen />
      </>,
    )

    fireEvent.click(screen.getByRole('button', { name: 'Expand companion' }))

    expect(screen.getByRole('dialog', { name: 'AI companion' })).toBeInTheDocument()
    expect(screen.getAllByText('Hello')).toHaveLength(2)
  })

  it('shows provider setup in fullscreen when no provider is available', () => {
    useAppStore.setState({ companionOpen: true, companionProviders: [] })
    render(
      <>
        <CompanionPanel />
        <CompanionFullscreen />
      </>,
    )

    fireEvent.click(screen.getByRole('button', { name: 'Expand companion' }))

    const dialog = screen.getByRole('dialog', { name: 'AI companion' })
    expect(within(dialog).getByText('Connect a local companion')).toBeInTheDocument()
    expect(within(dialog).getByText('opencode acp')).toBeInTheDocument()
    expect(
      within(dialog).getByText('npx --no-install @zed-industries/codex-acp'),
    ).toBeInTheDocument()
    expect(within(dialog).queryByRole('textbox', { name: 'Companion prompt' })).toBeNull()
  })

  it('uses one controller subscription when panel and fullscreen composers are mounted', () => {
    useAppStore.setState({ companionOpen: true, companionProviders: [availableProvider] })
    const assistant = useAppStore.getState().appendCompanionMessage('assistant', '', 'streaming')
    render(
      <>
        <CompanionPanel />
        <CompanionFullscreen />
      </>,
    )

    fireEvent.click(screen.getByRole('button', { name: 'Expand companion' }))

    expect(api.onCompanionUpdate).toHaveBeenCalledTimes(1)

    for (const [callback] of api.onCompanionUpdate.mock.calls) {
      callback({ type: 'assistant-delta', messageId: assistant.id, text: 'chunk' })
    }

    expect(
      useAppStore.getState().companionMessages.find((message) => message.id === assistant.id)
        ?.content,
    ).toBe('chunk')
  })

  it('shows companion errors as visible alerts', () => {
    useAppStore.setState({ companionOpen: true, companionError: 'Detection failed' })
    render(<CompanionPanel />)

    expect(screen.getByRole('alert')).toHaveTextContent('Detection failed')
  })

  it('submits with Enter and keeps Shift+Enter as a newline', () => {
    useAppStore.setState({ companionOpen: true, companionProviders: [availableProvider] })
    render(<CompanionPanel />)

    const prompt = screen.getByRole('textbox', { name: 'Companion prompt' })
    fireEvent.change(prompt, { target: { value: 'First line' } })
    fireEvent.keyDown(prompt, { key: 'Enter', shiftKey: true })

    expect(api.sendCompanionMessage).not.toHaveBeenCalled()

    fireEvent.keyDown(prompt, { key: 'Enter' })

    expect(api.sendCompanionMessage).toHaveBeenCalledWith(
      expect.objectContaining({ text: 'First line' }),
    )
  })
})
