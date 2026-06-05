import { describe, expect, it } from 'vitest'
import { useAppStore } from '../app-store'

describe('companion slice', () => {
  it('opens the panel and appends session-only messages', () => {
    useAppStore.getState().resetCompanion()

    useAppStore.getState().setCompanionOpen(true)
    const user = useAppStore.getState().appendCompanionMessage('user', 'What is Mdow?')
    const assistant = useAppStore.getState().appendCompanionMessage('assistant', '', 'streaming')
    useAppStore.getState().appendCompanionAssistantDelta(assistant.id, 'A reader.')

    expect(useAppStore.getState().companionOpen).toBe(true)
    expect(useAppStore.getState().companionMessages).toEqual([
      expect.objectContaining({ id: user.id, role: 'user', content: 'What is Mdow?' }),
      expect.objectContaining({ id: assistant.id, role: 'assistant', content: 'A reader.' }),
    ])
  })

  it('shares messages between compact and fullscreen state', () => {
    useAppStore.getState().resetCompanion()
    useAppStore.getState().appendCompanionMessage('user', 'Hello')
    useAppStore.getState().setCompanionFullscreen(true)

    expect(useAppStore.getState().companionFullscreen).toBe(true)
    expect(useAppStore.getState().companionMessages).toHaveLength(1)
  })
})
