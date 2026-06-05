import { renderHook, act } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { CompanionProviderStatus } from '../../../shared/types'
import { useAppStore } from '../store/app-store'
import { useCompanionController } from './useCompanionController'

const api = vi.hoisted(() => ({
  detectCompanionProviders: vi.fn((): Promise<CompanionProviderStatus[]> => Promise.resolve([])),
  getCompanionSettings: vi.fn(() => Promise.resolve({ provider: 'auto', customCommand: '' })),
  saveCompanionSettings: vi.fn(() => Promise.resolve()),
  saveAppState: vi.fn(() => Promise.resolve()),
  sendCompanionMessage: vi.fn(() => Promise.resolve()),
  cancelCompanionMessage: vi.fn(() => Promise.resolve()),
  onCompanionUpdate: vi.fn(() => () => {}),
}))

describe('useCompanionController', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    Object.defineProperty(window, 'api', { value: api, configurable: true })
    useAppStore.getState().resetCompanion()
    useAppStore.setState({ tabs: [], activeTabId: null, openFolderPath: null })
  })

  it('loads settings and providers on mount', async () => {
    api.detectCompanionProviders.mockResolvedValue([
      { id: 'opencode', label: 'opencode', command: 'opencode acp', status: 'available' },
    ])

    renderHook(() => useCompanionController())
    await act(async () => {})

    expect(useAppStore.getState().companionProvider).toBe('auto')
    expect(useAppStore.getState().companionProviders).toHaveLength(1)
  })

  it('appends user and assistant messages before sending to main', async () => {
    useAppStore.setState({
      tabs: [{ id: 'tab_1', path: '/docs/README.md', content: '# Mdow', scrollPosition: 0 }],
      activeTabId: 'tab_1',
      openFolderPath: '/docs',
    })
    const { result } = renderHook(() => useCompanionController())

    await act(async () => {
      await result.current.send('What is Mdow?')
    })

    expect(useAppStore.getState().companionMessages.map((message) => message.role)).toEqual([
      'user',
      'assistant',
    ])
    expect(api.sendCompanionMessage).toHaveBeenCalledWith(
      expect.objectContaining({
        text: 'What is Mdow?',
        activePath: '/docs/README.md',
        openFolderPath: '/docs',
      }),
    )
  })
})
