import { describe, expect, it, beforeEach } from 'vitest'
import { useAppStore } from '../app-store'

describe('Companion slice', () => {
  beforeEach(() => {
    useAppStore.getState().resetCompanionConversation()
    useAppStore.setState({
      companionPresentation: 'closed',
      companionTags: [],
      companionProviders: [],
      companionPreferredProvider: null,
      companionError: null,
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
