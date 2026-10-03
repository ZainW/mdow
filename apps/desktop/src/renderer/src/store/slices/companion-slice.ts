import type { StateCreator } from 'zustand'
import type {
  CompanionCitation,
  CompanionContextTag,
  CompanionMessage,
  CompanionModelState,
  CompanionPart,
  CompanionPermissionDecision,
  CompanionRuntimeStatus,
  CompanionUpdate,
} from '../../../../shared/types'

export type CompanionPresentation = 'closed' | 'drawer' | 'workspace'

const STALE_MODELS: CompanionModelState = {
  options: [],
  currentValue: null,
  stale: true,
  unavailableReason: 'Starting OpenCode…',
}

export interface CompanionSlice {
  companionPresentation: CompanionPresentation
  companionMessages: CompanionMessage[]
  companionStreaming: boolean
  /** null until the first status check finishes. */
  companionStatus: CompanionRuntimeStatus | null
  companionModelState: CompanionModelState
  companionTags: CompanionContextTag[]
  companionWarnings: string[]
  companionError: string | null
  setCompanionPresentation: (presentation: CompanionPresentation) => void
  toggleCompanion: () => void
  /** Checks for OpenCode, starts its server and loads the live model list. */
  connectCompanion: (directory?: string | null) => Promise<void>
  selectCompanionModel: (value: string) => Promise<void>
  setCompanionTags: (tags: CompanionContextTag[]) => void
  addCompanionTag: (tag: CompanionContextTag) => void
  removeCompanionTag: (sourceId: string) => void
  appendCompanionMessage: (message: CompanionMessage) => void
  beginCompanionRequest: () => void
  cancelCompanionRequest: () => void
  reviewCompanionChange: (
    permissionId: string,
    decision: CompanionPermissionDecision,
  ) => Promise<void>
  applyCompanionUpdate: (update: CompanionUpdate) => void
  clearCompanionError: () => void
  resetCompanionConversation: () => void
}

let streamingAssistantId: string | null = null

function ensureAssistant(messages: CompanionMessage[]): {
  messages: CompanionMessage[]
  id: string
} {
  if (streamingAssistantId) {
    return { messages, id: streamingAssistantId }
  }
  streamingAssistantId = crypto.randomUUID()
  return {
    id: streamingAssistantId,
    messages: [
      ...messages,
      {
        id: streamingAssistantId,
        role: 'assistant',
        content: '',
        parts: [],
        status: 'streaming',
        citations: [],
      },
    ],
  }
}

function mapAssistant(
  messages: CompanionMessage[],
  id: string,
  map: (message: CompanionMessage) => CompanionMessage,
): CompanionMessage[] {
  return messages.map((m) => (m.id === id ? map(m) : m))
}

function upsertPart(parts: CompanionPart[], part: CompanionPart): CompanionPart[] {
  if (part.kind === 'text') {
    const last = parts.at(-1)
    if (last?.kind === 'text') {
      return [...parts.slice(0, -1), { kind: 'text', text: last.text + part.text }]
    }
    return [...parts, part]
  }
  if (part.kind === 'thinking') {
    // Reasoning that resumes right after earlier reasoning reads as one block, not a stack of rows.
    const last = parts.at(-1)
    if (last?.kind === 'thinking' && last.done) {
      return [
        ...parts.slice(0, -1),
        { kind: 'thinking', text: `${last.text}\n\n${part.text}`, done: part.done },
      ]
    }
    const idx = parts.findLastIndex((p) => p.kind === 'thinking')
    if (idx >= 0) {
      const existing = parts[idx]
      if (existing.kind !== 'thinking' || existing.done) return [...parts, part]
      const next = [...parts]
      next[idx] = { kind: 'thinking', text: existing.text + part.text, done: part.done }
      return next
    }
    return [...parts, part]
  }
  if (part.kind === 'tool') {
    const idx = parts.findIndex((p) => p.kind === 'tool' && p.toolCallId === part.toolCallId)
    if (idx >= 0) {
      const existing = parts[idx]
      if (existing.kind !== 'tool') return [...parts, part]
      const next = [...parts]
      next[idx] = {
        ...existing,
        ...part,
        input: part.input ?? existing.input,
        output: part.output ?? existing.output,
        error: part.error ?? existing.error,
      }
      return next
    }
    return [...parts, part]
  }
  if (part.kind === 'change') {
    const idx = parts.findIndex((p) => p.kind === 'change' && p.toolCallId === part.toolCallId)
    if (idx >= 0) {
      const existing = parts[idx]
      if (existing.kind !== 'change') return [...parts, part]
      const next = [...parts]
      next[idx] = {
        ...existing,
        status: part.status,
        // The review id only means something while the change waits for a decision.
        permissionId:
          part.status === 'pending' ? (part.permissionId ?? existing.permissionId) : undefined,
        files: part.files.length > 0 ? part.files : existing.files,
        error: part.error,
      }
      return next
    }
    return [...parts, part]
  }
  return [...parts, part]
}

function textFromParts(parts: CompanionPart[]): string {
  return parts
    .filter((p): p is Extract<CompanionPart, { kind: 'text' }> => p.kind === 'text')
    .map((p) => p.text)
    .join('')
}

/** Settles parts left open when a turn ends early: reasoning closes, reviews expire. */
function settleParts(parts: CompanionPart[]): CompanionPart[] {
  return parts.map((part) => {
    if (part.kind === 'thinking') return { ...part, done: true }
    if (part.kind === 'change' && part.status === 'pending') {
      return { ...part, status: 'rejected', permissionId: undefined }
    }
    if (part.kind === 'tool' && (part.state === 'pending' || part.state === 'running')) {
      return { ...part, state: 'cancelled' }
    }
    return part
  })
}

function endTurn(
  messages: CompanionMessage[],
  status: NonNullable<CompanionMessage['status']>,
  messageId?: string,
): CompanionMessage[] {
  return messages.map((message) =>
    message.id === messageId || message.status === 'streaming'
      ? { ...message, status, parts: settleParts(message.parts) }
      : message,
  )
}

export const createCompanionSlice: StateCreator<CompanionSlice, [], [], CompanionSlice> = (
  set,
  get,
) => ({
  companionPresentation: 'closed',
  companionMessages: [],
  companionStreaming: false,
  companionStatus: null,
  companionModelState: STALE_MODELS,
  companionTags: [],
  companionWarnings: [],
  companionError: null,

  setCompanionPresentation: (presentation) => set({ companionPresentation: presentation }),
  toggleCompanion: () =>
    set((state) => ({
      companionPresentation: state.companionPresentation === 'closed' ? 'drawer' : 'closed',
    })),
  connectCompanion: async (directory) => {
    try {
      const status = await window.api.startCompanion()
      set({ companionStatus: status })
      if (status.availability !== 'available') {
        set({ companionModelState: { ...STALE_MODELS, unavailableReason: status.detail } })
        return
      }
      const modelState = await window.api.getCompanionModels(directory ?? null)
      set({ companionModelState: modelState })
    } catch (error) {
      set({
        companionStatus: {
          availability: 'failed',
          detail: error instanceof Error ? error.message : 'Could not start OpenCode.',
        },
      })
    }
  },
  selectCompanionModel: async (value) => {
    const previous = get().companionModelState
    set({ companionModelState: { ...previous, currentValue: value } })
    try {
      const modelState = await window.api.setCompanionModel(value)
      set({ companionModelState: modelState, companionError: null })
    } catch (error) {
      set({
        companionModelState: previous,
        companionError: error instanceof Error ? error.message : 'Could not change model',
      })
    }
  },
  setCompanionTags: (tags) => set({ companionTags: tags }),
  addCompanionTag: (tag) =>
    set((state) => {
      if (state.companionTags.some((t) => t.sourceId === tag.sourceId)) return state
      return { companionTags: [...state.companionTags, tag] }
    }),
  removeCompanionTag: (sourceId) =>
    set((state) => ({
      companionTags: state.companionTags.filter((t) => t.sourceId !== sourceId),
    })),
  appendCompanionMessage: (message) =>
    set((state) => ({
      companionMessages: [
        ...state.companionMessages,
        {
          ...message,
          parts:
            message.parts ?? (message.content ? [{ kind: 'text', text: message.content }] : []),
        },
      ],
      companionError: message.role === 'user' ? null : state.companionError,
    })),
  beginCompanionRequest: () =>
    set((state) => {
      if (state.companionStreaming) return state
      const ensured = ensureAssistant(state.companionMessages)
      return {
        companionStreaming: true,
        companionError: null,
        companionMessages: mapAssistant(ensured.messages, ensured.id, (message) => ({
          ...message,
          status: 'streaming',
        })),
      }
    }),
  cancelCompanionRequest: () => {
    streamingAssistantId = null
    set((state) => ({
      companionStreaming: false,
      companionMessages: endTurn(state.companionMessages, 'cancelled'),
    }))
  },
  reviewCompanionChange: async (permissionId, decision) => {
    const change = get()
      .companionMessages.flatMap((message) => message.parts)
      .find(
        (part): part is Extract<CompanionPart, { kind: 'change' }> =>
          part.kind === 'change' && part.permissionId === permissionId,
      )
    const toolCallId = change?.toolCallId
    if (!toolCallId) return
    const markReviewed = (status: 'pending' | 'rejected', keepId: boolean) =>
      set((state) => ({
        companionMessages: state.companionMessages.map((message) => ({
          ...message,
          parts: message.parts.map((part) =>
            part.kind === 'change' && part.toolCallId === toolCallId
              ? { ...part, status, permissionId: keepId ? permissionId : undefined }
              : part,
          ),
        })),
      }))
    // Hide the buttons right away; OpenCode confirms with applied or rejected events.
    markReviewed(decision === 'reject' ? 'rejected' : 'pending', false)
    try {
      await window.api.replyCompanionPermission(permissionId, decision)
    } catch (error) {
      markReviewed('pending', true)
      set({ companionError: error instanceof Error ? error.message : 'Could not send review' })
    }
  },
  applyCompanionUpdate: (update) => {
    switch (update.kind) {
      case 'delta':
        set((state) => {
          const ensured = ensureAssistant(state.companionMessages)
          return {
            companionStreaming: true,
            companionMessages: mapAssistant(ensured.messages, ensured.id, (m) => {
              const parts = upsertPart(m.parts, { kind: 'text', text: update.text })
              return { ...m, parts, content: textFromParts(parts), status: 'streaming' }
            }),
          }
        })
        break
      case 'thinking':
        set((state) => {
          const ensured = ensureAssistant(state.companionMessages)
          return {
            companionStreaming: true,
            companionMessages: mapAssistant(ensured.messages, ensured.id, (m) => ({
              ...m,
              parts: upsertPart(m.parts, { kind: 'thinking', text: update.text, done: false }),
              status: 'streaming',
            })),
          }
        })
        break
      case 'thinking-done':
        set((state) => {
          if (!streamingAssistantId) return state
          return {
            companionMessages: mapAssistant(state.companionMessages, streamingAssistantId, (m) => ({
              ...m,
              parts: m.parts.map((p) => (p.kind === 'thinking' ? { ...p, done: true } : p)),
            })),
          }
        })
        break
      case 'tool':
        set((state) => {
          const ensured = ensureAssistant(state.companionMessages)
          return {
            companionStreaming: true,
            companionMessages: mapAssistant(ensured.messages, ensured.id, (m) => ({
              ...m,
              parts: upsertPart(m.parts, {
                kind: 'tool',
                toolCallId: update.toolCallId,
                name: update.name,
                state: update.state,
                input: update.input,
                output: update.output,
                error: update.error,
              }),
              status: 'streaming',
            })),
          }
        })
        break
      case 'change':
        set((state) => {
          const ensured = ensureAssistant(state.companionMessages)
          return {
            companionStreaming: true,
            companionMessages: mapAssistant(ensured.messages, ensured.id, (m) => ({
              ...m,
              parts: upsertPart(m.parts, {
                kind: 'change',
                toolCallId: update.toolCallId,
                permissionId: update.permissionId,
                status: update.status,
                files: update.files ?? [],
                error: update.error,
              }),
              status: 'streaming',
            })),
          }
        })
        break
      case 'status':
        set((state) => {
          const ensured = ensureAssistant(state.companionMessages)
          return {
            companionMessages: mapAssistant(ensured.messages, ensured.id, (m) => ({
              ...m,
              parts: upsertPart(m.parts, { kind: 'status', message: update.message }),
            })),
          }
        })
        break
      case 'citation':
        set((state) => {
          if (!streamingAssistantId) return state
          return {
            companionMessages: mapAssistant(state.companionMessages, streamingAssistantId, (m) => {
              if (m.citations?.some((c) => c.sourceId === update.citation.sourceId)) return m
              const citations: CompanionCitation[] = [...(m.citations ?? []), update.citation]
              return { ...m, citations }
            }),
          }
        })
        break
      case 'warning':
        set((state) => ({
          companionWarnings: [...state.companionWarnings, update.message],
        }))
        break
      case 'error':
        streamingAssistantId = null
        set((state) => ({
          companionStreaming: false,
          companionError: update.message,
          companionMessages: endTurn(state.companionMessages, 'error'),
        }))
        break
      case 'done':
        streamingAssistantId = null
        set((state) => ({
          companionStreaming: false,
          companionMessages: endTurn(state.companionMessages, 'complete', update.messageId),
        }))
        break
      case 'cancelled':
        streamingAssistantId = null
        set((state) => ({
          companionStreaming: false,
          companionMessages: endTurn(state.companionMessages, 'cancelled', update.messageId),
        }))
        break
      default: {
        const exhaustive: never = update
        void exhaustive
      }
    }
  },
  clearCompanionError: () => set({ companionError: null }),
  resetCompanionConversation: () => {
    streamingAssistantId = null
    set({
      companionMessages: [],
      companionStreaming: false,
      companionWarnings: [],
      companionError: null,
    })
    if (typeof window !== 'undefined' && window.api?.resetCompanion) {
      void window.api.resetCompanion()
    }
  },
})
