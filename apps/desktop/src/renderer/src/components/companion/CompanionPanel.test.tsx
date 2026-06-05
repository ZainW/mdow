import { fireEvent, render, screen } from '@testing-library/react'
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

  it('shows provider setup when no provider is available', () => {
    useAppStore.setState({ companionOpen: true, companionProviders: [] })
    render(<CompanionPanel />)

    expect(screen.getByText('Connect a local companion')).toBeInTheDocument()
    expect(screen.getByText('opencode acp')).toBeInTheDocument()
    expect(screen.getByText('npx --no-install @zed-industries/codex-acp')).toBeInTheDocument()
  })

  it('expands and collapses the same conversation', () => {
    useAppStore.setState({ companionOpen: true })
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
