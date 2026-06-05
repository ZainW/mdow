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
    vi.resetAllMocks()
    api.detectCompanionProviders.mockResolvedValue([])
    api.getCompanionSettings.mockResolvedValue({ provider: 'auto', customCommand: '' })
    api.saveCompanionSettings.mockResolvedValue(undefined)
    api.saveAppState.mockResolvedValue(undefined)
    api.sendCompanionMessage.mockResolvedValue(undefined)
    api.cancelCompanionMessage.mockResolvedValue(undefined)
    api.onCompanionUpdate.mockReturnValue(() => {})
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

  it('does not finalize a cancelled assistant message when send later resolves', async () => {
    const send = deferred<void>()
    api.sendCompanionMessage.mockReturnValue(send.promise)
    const { result } = renderHook(() => useCompanionController())

    const sendPromise = act(async () => {
      await result.current.send('Summarize this')
    })
    await act(async () => {})

    const assistant = useAppStore
      .getState()
      .companionMessages.find((message) => message.role === 'assistant')
    expect(assistant).toBeDefined()

    await act(async () => {
      useAppStore.getState().appendCompanionAssistantDelta(assistant!.id, 'Partial answer')
      await result.current.cancel()
    })

    send.resolve()
    await sendPromise

    expect(
      useAppStore.getState().companionMessages.find((message) => message.id === assistant!.id)
        ?.status,
    ).not.toBe('complete')
  })

  it('ignores overlapping sends without losing cancellation tracking', async () => {
    const firstSend = deferred<void>()
    api.sendCompanionMessage
      .mockReturnValueOnce(firstSend.promise)
      .mockRejectedValueOnce(new Error('busy'))
    const { result } = renderHook(() => useCompanionController())

    const firstSendPromise = act(async () => {
      await result.current.send('First question')
    })
    await act(async () => {})

    await act(async () => {
      await result.current.send('Second question')
    })

    expect(api.sendCompanionMessage).toHaveBeenCalledTimes(1)
    expect(useAppStore.getState().companionMessages.map((message) => message.role)).toEqual([
      'user',
      'assistant',
    ])

    const assistant = useAppStore
      .getState()
      .companionMessages.find((message) => message.role === 'assistant')
    expect(assistant).toBeDefined()

    await act(async () => {
      useAppStore.getState().appendCompanionAssistantDelta(assistant!.id, 'Partial answer')
      await result.current.cancel()
    })

    firstSend.resolve()
    await firstSendPromise

    expect(
      useAppStore.getState().companionMessages.find((message) => message.id === assistant!.id)
        ?.status,
    ).not.toBe('complete')
  })

  it('does not finalize an in-flight assistant message after unmount and remount', async () => {
    const send = deferred<void>()
    api.sendCompanionMessage.mockReturnValue(send.promise)
    const { result, unmount } = renderHook(() => useCompanionController())

    const sendPromise = result.current.send('Summarize this')
    await act(async () => {})

    const assistant = useAppStore
      .getState()
      .companionMessages.find((message) => message.role === 'assistant')
    expect(assistant).toBeDefined()

    unmount()
    expect(
      useAppStore.getState().companionMessages.find((message) => message.id === assistant!.id)
        ?.status,
    ).toBe('error')

    renderHook(() => useCompanionController())
    await act(async () => {})
    act(() => {
      useAppStore.getState().appendCompanionAssistantDelta(assistant!.id, 'Partial answer')
    })

    send.resolve()
    await sendPromise

    expect(
      useAppStore.getState().companionMessages.find((message) => message.id === assistant!.id)
        ?.status,
    ).toBe('error')
  })
})

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void
  let reject!: (reason?: unknown) => void
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise
    reject = rejectPromise
  })
  return { promise, resolve, reject }
}
