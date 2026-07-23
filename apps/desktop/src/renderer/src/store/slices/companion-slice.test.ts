import { describe, expect, it, beforeEach } from 'vitest'
import { useAppStore } from '../app-store'

describe('Companion slice', () => {
  beforeEach(() => {
    useAppStore.getState().resetCompanionConversation()
    useAppStore.setState({
      companionOpen: false,
      companionFullscreen: false,
      companionTags: [],
      companionProviders: [],
      companionPreferredProvider: null,
      companionError: null,
    })
  })

  it('toggles open without changing unrelated UI flags', () => {
    const beforeSidebar = useAppStore.getState().sidebarMode
    useAppStore.getState().toggleCompanion()
    expect(useAppStore.getState().companionOpen).toBe(true)
    expect(useAppStore.getState().sidebarMode).toBe(beforeSidebar)
  })

  it('applies streaming deltas into one assistant message', () => {
    useAppStore.getState().applyCompanionUpdate({ kind: 'delta', text: 'Hello' })
    useAppStore.getState().applyCompanionUpdate({ kind: 'delta', text: ' world' })
    const messages = useAppStore.getState().companionMessages
    expect(messages).toHaveLength(1)
    expect(messages[0].content).toBe('Hello world')
    expect(messages[0].status).toBe('streaming')
    expect(useAppStore.getState().companionStreaming).toBe(true)

    useAppStore.getState().applyCompanionUpdate({ kind: 'done', messageId: messages[0].id })
    expect(useAppStore.getState().companionStreaming).toBe(false)
    expect(useAppStore.getState().companionMessages[0].status).toBe('complete')
  })

  it('dedupes companion tags by sourceId', () => {
    const tag = { kind: 'file' as const, path: '/docs/a.md', sourceId: 'tag:/docs/a.md' }
    useAppStore.getState().addCompanionTag(tag)
    useAppStore.getState().addCompanionTag(tag)
    expect(useAppStore.getState().companionTags).toHaveLength(1)
  })
})
