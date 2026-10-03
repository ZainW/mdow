import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { CompanionFileChange } from '../../../../shared/types'
import { useAppStore } from '../app-store'
import { stubWindowApi } from '../../test/stubWindowApi'

const liveModels = {
  options: [
    {
      value: 'opencode/claude-sonnet-5-5',
      name: 'Claude Sonnet 5.5',
      providerId: 'opencode',
      providerName: 'OpenCode Zen',
    },
  ],
  currentValue: 'opencode/claude-sonnet-5-5',
  stale: false,
}

const startCompanion = vi.fn()
const getCompanionModels = vi.fn()
const setCompanionModel = vi.fn()
const replyCompanionPermission = vi.fn()
const resetCompanion = vi.fn()

stubWindowApi(() => ({
  startCompanion,
  getCompanionModels,
  setCompanionModel,
  replyCompanionPermission,
  resetCompanion,
}))

const file: CompanionFileChange = {
  path: '/docs/a.md',
  displayPath: 'a.md',
  patch: '@@ -1 +1 @@\n-old\n+new\n',
  additions: 1,
  deletions: 1,
  status: 'modified',
}

describe('Companion slice', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    startCompanion.mockResolvedValue({ availability: 'available', version: '2.0.16' })
    getCompanionModels.mockResolvedValue(liveModels)
    setCompanionModel.mockResolvedValue(liveModels)
    replyCompanionPermission.mockResolvedValue(undefined)
    resetCompanion.mockResolvedValue(undefined)
    useAppStore.getState().resetCompanionConversation()
    useAppStore.setState({
      companionPresentation: 'closed',
      companionTags: [],
      companionError: null,
      companionStatus: null,
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
    useAppStore.getState().setCompanionPresentation('drawer')
    expect(useAppStore.getState().companionMessages[0]?.content).toBe('Keep this message')
  })

  it('connects to OpenCode and loads models for the open folder', async () => {
    await useAppStore.getState().connectCompanion('/docs')
    expect(getCompanionModels).toHaveBeenCalledWith('/docs')
    expect(useAppStore.getState().companionStatus).toEqual({
      availability: 'available',
      version: '2.0.16',
    })
    expect(useAppStore.getState().companionModelState).toEqual(liveModels)
  })

  it('reports a missing OpenCode without asking for models', async () => {
    startCompanion.mockResolvedValueOnce({ availability: 'missing', detail: 'Install OpenCode' })
    await useAppStore.getState().connectCompanion(null)
    expect(getCompanionModels).not.toHaveBeenCalled()
    expect(useAppStore.getState().companionStatus?.availability).toBe('missing')
  })

  it('rolls back a model choice OpenCode refuses', async () => {
    useAppStore.setState({ companionModelState: liveModels })
    setCompanionModel.mockRejectedValueOnce(new Error('Model is not available'))
    await useAppStore.getState().selectCompanionModel('opencode/gone')
    expect(useAppStore.getState().companionModelState.currentValue).toBe(
      'opencode/claude-sonnet-5-5',
    )
    expect(useAppStore.getState().companionError).toBe('Model is not available')
  })

  it('applies streaming deltas into one assistant message', () => {
    useAppStore.getState().applyCompanionUpdate({ kind: 'delta', text: 'Hello' })
    useAppStore.getState().applyCompanionUpdate({ kind: 'delta', text: ' world' })
    const messages = useAppStore.getState().companionMessages
    expect(messages).toHaveLength(1)
    expect(messages[0].content).toBe('Hello world')
    expect(useAppStore.getState().companionStreaming).toBe(true)

    useAppStore.getState().applyCompanionUpdate({ kind: 'done', messageId: messages[0].id })
    expect(useAppStore.getState().companionStreaming).toBe(false)
    expect(useAppStore.getState().companionMessages[0].status).toBe('complete')
  })

  it('keeps thinking, tools and answer text as separate parts', () => {
    const apply = useAppStore.getState().applyCompanionUpdate
    apply({ kind: 'thinking', text: 'hmm' })
    apply({ kind: 'thinking', text: '…' })
    apply({ kind: 'thinking-done' })
    apply({ kind: 'tool', toolCallId: 't1', name: 'Read a.md', state: 'running', input: '{}' })
    apply({ kind: 'tool', toolCallId: 't1', name: 'Read a.md', state: 'completed', output: 'ok' })
    apply({ kind: 'delta', text: 'Answer' })

    expect(useAppStore.getState().companionMessages[0].parts).toEqual([
      { kind: 'thinking', text: 'hmm…', done: true },
      {
        kind: 'tool',
        toolCallId: 't1',
        name: 'Read a.md',
        state: 'completed',
        input: '{}',
        output: 'ok',
      },
      { kind: 'text', text: 'Answer' },
    ])
  })

  it('merges reasoning that resumes right after earlier reasoning', () => {
    const apply = useAppStore.getState().applyCompanionUpdate
    apply({ kind: 'thinking', text: 'First' })
    apply({ kind: 'thinking-done' })
    apply({ kind: 'thinking', text: 'Second' })
    expect(useAppStore.getState().companionMessages[0].parts).toEqual([
      { kind: 'thinking', text: 'First\n\nSecond', done: false },
    ])
  })

  it('tracks a change from review to applied, keeping the diff', () => {
    const apply = useAppStore.getState().applyCompanionUpdate
    apply({
      kind: 'change',
      toolCallId: 'c1',
      permissionId: 'per_1',
      status: 'pending',
      files: [file],
    })
    apply({ kind: 'change', toolCallId: 'c1', status: 'applied' })

    expect(useAppStore.getState().companionMessages[0].parts).toEqual([
      {
        kind: 'change',
        toolCallId: 'c1',
        permissionId: undefined,
        status: 'applied',
        files: [file],
        error: undefined,
      },
    ])
  })

  it('sends a review decision and hides the buttons right away', async () => {
    useAppStore.getState().applyCompanionUpdate({
      kind: 'change',
      toolCallId: 'c1',
      permissionId: 'per_1',
      status: 'pending',
      files: [file],
    })

    const pending = useAppStore.getState().reviewCompanionChange('per_1', 'reject')
    expect(useAppStore.getState().companionMessages[0].parts[0]).toMatchObject({
      status: 'rejected',
      permissionId: undefined,
    })
    await pending
    expect(replyCompanionPermission).toHaveBeenCalledWith('per_1', 'reject')
  })

  it('restores the review buttons when the reply fails', async () => {
    replyCompanionPermission.mockRejectedValueOnce(new Error('no longer waiting'))
    useAppStore.getState().applyCompanionUpdate({
      kind: 'change',
      toolCallId: 'c1',
      permissionId: 'per_1',
      status: 'pending',
      files: [file],
    })
    await useAppStore.getState().reviewCompanionChange('per_1', 'approve')
    expect(useAppStore.getState().companionMessages[0].parts[0]).toMatchObject({
      status: 'pending',
      permissionId: 'per_1',
    })
    expect(useAppStore.getState().companionError).toBe('no longer waiting')
  })

  it('settles open reviews and tools when a turn is stopped', () => {
    const apply = useAppStore.getState().applyCompanionUpdate
    apply({ kind: 'tool', toolCallId: 't1', name: 'Read a.md', state: 'running' })
    apply({ kind: 'change', toolCallId: 'c1', permissionId: 'per_1', status: 'pending', files: [] })
    const messageId = useAppStore.getState().companionMessages[0].id
    apply({ kind: 'cancelled', messageId })

    const message = useAppStore.getState().companionMessages[0]
    expect(message.status).toBe('cancelled')
    expect(message.parts).toEqual([
      expect.objectContaining({ kind: 'tool', state: 'cancelled' }),
      expect.objectContaining({ kind: 'change', status: 'rejected', permissionId: undefined }),
    ])
  })

  it('dedupes citations and tags', () => {
    const apply = useAppStore.getState().applyCompanionUpdate
    apply({ kind: 'delta', text: 'x' })
    const citation = { sourceId: 'read:/docs/a.md', path: '/docs/a.md', label: 'a.md' }
    apply({ kind: 'citation', citation })
    apply({ kind: 'citation', citation })
    expect(useAppStore.getState().companionMessages[0].citations).toHaveLength(1)

    const tag = { kind: 'file' as const, path: '/docs/a.md', sourceId: 'tag:/docs/a.md' }
    useAppStore.getState().addCompanionTag(tag)
    useAppStore.getState().addCompanionTag(tag)
    expect(useAppStore.getState().companionTags).toHaveLength(1)
  })

  it('starts a fresh OpenCode session on reset', () => {
    useAppStore.getState().applyCompanionUpdate({ kind: 'delta', text: 'x' })
    useAppStore.getState().resetCompanionConversation()
    expect(useAppStore.getState().companionMessages).toEqual([])
    expect(resetCompanion).toHaveBeenCalled()
  })
})
