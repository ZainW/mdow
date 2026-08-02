import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useAppStore } from '../app-store'
import { stubWindowApi } from '../../test/stubWindowApi'

const liveModels = {
  options: [
    {
      value: 'openai/gpt-5.4',
      name: 'GPT-5.4',
      provider: 'openai' as const,
    },
  ],
  currentValue: 'openai/gpt-5.4',
  stale: false,
}

const getCompanionModels = vi.fn().mockResolvedValue(liveModels)
const setCompanionModel = vi.fn().mockResolvedValue(liveModels)

stubWindowApi(() => ({
  saveCompanionSettings: vi.fn().mockResolvedValue(undefined),
  startCompanionSession: vi.fn().mockResolvedValue({ ok: true, providerId: 'opencode' }),
  getCompanionModels,
  setCompanionModel,
}))

describe('Companion slice', () => {
  beforeEach(() => {
    useAppStore.getState().resetCompanionConversation()
    useAppStore.setState({
      companionPresentation: 'closed',
      companionTags: [],
      companionProviders: [],
      companionPreferredProvider: null,
      companionError: null,
      companionContextTrace: null,
      companionModelState: {
        options: [],
        currentValue: null,
        stale: true,
        unavailableReason: 'Start Companion to load models',
      },
    })
  })

  it('toggles the drawer without changing unrelated UI flags', () => {
    const beforeSidebar = useAppStore.getState().sidebarMode
    useAppStore.getState().toggleCompanion()
    expect(useAppStore.getState().companionPresentation).toBe('drawer')
    expect(useAppStore.getState().sidebarMode).toBe(beforeSidebar)

    useAppStore.getState().toggleCompanion()
    expect(useAppStore.getState().companionPresentation).toBe('closed')
  })

  it('moves between the drawer and workspace without resetting the conversation', () => {
    useAppStore.getState().appendCompanionMessage({
      id: 'user-1',
      role: 'user',
      content: 'Keep this message',
      parts: [{ kind: 'text', text: 'Keep this message' }],
      status: 'complete',
    })

    useAppStore.getState().setCompanionPresentation('workspace')
    expect(useAppStore.getState().companionPresentation).toBe('workspace')

    useAppStore.getState().setCompanionPresentation('drawer')
    expect(useAppStore.getState().companionPresentation).toBe('drawer')
    expect(useAppStore.getState().companionMessages[0]?.content).toBe('Keep this message')
  })

  it('stores the honest context trace from the main process', () => {
    const trace = {
      focusedCount: 1,
      attachedCount: 0,
      searchedCount: 0,
      readRangeCount: 0,
      injectedBytes: 400,
      estimatedTokens: 100,
      retrievalMode: 'focused-only' as const,
      items: [{ path: '/docs/a.md', reason: 'focused' as const, bytes: 400 }],
    }

    useAppStore.getState().applyCompanionUpdate({
      kind: 'context',
      summary: '1 focused',
      warnings: [],
      trace,
    })

    expect(useAppStore.getState().companionContextTrace).toEqual(trace)
  })

  it('loads and changes live model state through the typed bridge', async () => {
    await useAppStore.getState().loadCompanionModels()
    expect(useAppStore.getState().companionModelState).toEqual(liveModels)

    await useAppStore.getState().selectCompanionModel('openai/gpt-5.4')
    expect(setCompanionModel).toHaveBeenCalledWith('openai/gpt-5.4')
    expect(useAppStore.getState().companionModelState.currentValue).toBe('openai/gpt-5.4')
  })

  it('applies streaming deltas into one assistant message', () => {
    useAppStore.getState().applyCompanionUpdate({ kind: 'delta', text: 'Hello' })
    useAppStore.getState().applyCompanionUpdate({ kind: 'delta', text: ' world' })
    const messages = useAppStore.getState().companionMessages
    expect(messages).toHaveLength(1)
    expect(messages[0].content).toBe('Hello world')
    expect(messages[0].parts).toEqual([{ kind: 'text', text: 'Hello world' }])
    expect(messages[0].status).toBe('streaming')
    expect(useAppStore.getState().companionStreaming).toBe(true)

    useAppStore.getState().applyCompanionUpdate({ kind: 'done', messageId: messages[0].id })
    expect(useAppStore.getState().companionStreaming).toBe(false)
    expect(useAppStore.getState().companionMessages[0].status).toBe('complete')
  })

  it('enters a cancellable request state before the first agent update', () => {
    useAppStore.getState().beginCompanionRequest()

    const state = useAppStore.getState()
    expect(state.companionStreaming).toBe(true)
    expect(state.companionMessages).toHaveLength(1)
    expect(state.companionMessages[0]).toMatchObject({
      role: 'assistant',
      status: 'streaming',
      parts: [],
    })
  })

  it('marks the active assistant response cancelled', () => {
    useAppStore.getState().beginCompanionRequest()
    const messageId = useAppStore.getState().companionMessages[0].id

    useAppStore.getState().applyCompanionUpdate({ kind: 'cancelled', messageId })

    expect(useAppStore.getState().companionStreaming).toBe(false)
    expect(useAppStore.getState().companionMessages[0].status).toBe('cancelled')
  })

  it('keeps thinking and tool parts separate from answer text', () => {
    useAppStore.getState().applyCompanionUpdate({ kind: 'thinking', text: 'hmm' })
    useAppStore.getState().applyCompanionUpdate({ kind: 'thinking', text: '…' })
    useAppStore.getState().applyCompanionUpdate({ kind: 'thinking-done' })
    useAppStore.getState().applyCompanionUpdate({
      kind: 'tool',
      toolCallId: 't1',
      name: 'read',
      state: 'running',
      input: '{"path":"a.md"}',
    })
    useAppStore.getState().applyCompanionUpdate({
      kind: 'tool',
      toolCallId: 't1',
      name: 'read',
      state: 'completed',
      output: 'ok',
    })
    useAppStore.getState().applyCompanionUpdate({ kind: 'delta', text: 'Answer' })

    const message = useAppStore.getState().companionMessages[0]
    expect(message.parts).toEqual([
      { kind: 'thinking', text: 'hmm…', done: true },
      {
        kind: 'tool',
        toolCallId: 't1',
        name: 'read',
        state: 'completed',
        input: '{"path":"a.md"}',
        output: 'ok',
      },
      { kind: 'text', text: 'Answer' },
    ])
    expect(message.content).toBe('Answer')
  })

  it('dedupes companion tags by sourceId', () => {
    const tag = { kind: 'file' as const, path: '/docs/a.md', sourceId: 'tag:/docs/a.md' }
    useAppStore.getState().addCompanionTag(tag)
    useAppStore.getState().addCompanionTag(tag)
    expect(useAppStore.getState().companionTags).toHaveLength(1)
  })
})
