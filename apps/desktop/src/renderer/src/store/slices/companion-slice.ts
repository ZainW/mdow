import type { StateCreator } from 'zustand'
import type {
  CompanionCitation,
  CompanionContextTag,
  CompanionMessage,
  CompanionPart,
  CompanionProviderId,
  CompanionProviderStatus,
  CompanionUpdate,
} from '../../../../shared/types'

export interface CompanionSlice {
  companionOpen: boolean
  companionFullscreen: boolean
  companionMessages: CompanionMessage[]
  companionStreaming: boolean
  companionProviders: CompanionProviderStatus[]
  companionPreferredProvider: CompanionProviderId | null
  companionCustomCommand: string
  companionContextSummary: string
  companionWarnings: string[]
  companionTags: CompanionContextTag[]
  companionError: string | null
  setCompanionOpen: (open: boolean) => void
  toggleCompanion: () => void
  setCompanionFullscreen: (open: boolean) => void
  setCompanionProviders: (providers: CompanionProviderStatus[]) => void
  setCompanionPreferredProvider: (id: CompanionProviderId | null) => void
  setCompanionCustomCommand: (command: string) => void
  setCompanionTags: (tags: CompanionContextTag[]) => void
  addCompanionTag: (tag: CompanionContextTag) => void
  removeCompanionTag: (sourceId: string) => void
  appendCompanionMessage: (message: CompanionMessage) => void
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
    const idx = parts.findLastIndex((p) => p.kind === 'thinking')
    if (idx >= 0) {
      const existing = parts[idx]
      if (existing.kind !== 'thinking') return [...parts, part]
      if (existing.done) return [...parts, part]
      const next = [...parts]
      next[idx] = {
        kind: 'thinking',
        text: existing.text + part.text,
        done: part.done,
      }
      return next
    }
    return [...parts, part]
  }
  if (part.kind === 'tool') {
    const idx = parts.findIndex((p) => p.kind === 'tool' && p.toolCallId === part.toolCallId)
    if (idx >= 0) {
      const next = [...parts]
      const existing = parts[idx]
      if (existing.kind !== 'tool') return [...parts, part]
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
  return [...parts, part]
}

function textFromParts(parts: CompanionPart[]): string {
  return parts
    .filter((p): p is Extract<CompanionPart, { kind: 'text' }> => p.kind === 'text')
    .map((p) => p.text)
    .join('')
}

export const createCompanionSlice: StateCreator<CompanionSlice, [], [], CompanionSlice> = (
  set,
  get,
) => ({
  companionOpen: false,
  companionFullscreen: false,
  companionMessages: [],
  companionStreaming: false,
  companionProviders: [],
  companionPreferredProvider: null,
  companionCustomCommand: '',
  companionContextSummary: '',
  companionWarnings: [],
  companionTags: [],
  companionError: null,

  setCompanionOpen: (open) =>
    set({
      companionOpen: open,
      companionFullscreen: open ? get().companionFullscreen : false,
    }),
  toggleCompanion: () =>
    set((state) => ({
      companionOpen: !state.companionOpen,
      companionFullscreen: state.companionOpen ? false : state.companionFullscreen,
    })),
  setCompanionFullscreen: (open) =>
    set({
      companionFullscreen: open,
      companionOpen: open || get().companionOpen,
    }),
  setCompanionProviders: (providers) => set({ companionProviders: providers }),
  setCompanionPreferredProvider: (id) => {
    if (typeof window !== 'undefined' && window.api) {
      void window.api.saveCompanionSettings({ preferredProvider: id })
    }
    set({ companionPreferredProvider: id })
  },
  setCompanionCustomCommand: (command) => {
    if (typeof window !== 'undefined' && window.api) {
      void window.api.saveCompanionSettings({ customCommand: command })
    }
    set({ companionCustomCommand: command })
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
          companionMessages: state.companionMessages.map((m) =>
            m.status === 'streaming' ? { ...m, status: 'error' } : m,
          ),
        }))
        break
      case 'done':
        streamingAssistantId = null
        set((state) => ({
          companionStreaming: false,
          companionMessages: state.companionMessages.map((m) =>
            m.id === update.messageId || m.status === 'streaming'
              ? {
                  ...m,
                  status: 'complete',
                  parts: m.parts.map((p) => (p.kind === 'thinking' ? { ...p, done: true } : p)),
                }
              : m,
          ),
        }))
        break
      case 'context':
        set({
          companionContextSummary: update.summary,
          companionWarnings: update.warnings,
        })
        break
      default: {
        const _exhaustive: never = update
        void _exhaustive
      }
    }
  },
  clearCompanionError: () => set({ companionError: null }),
  resetCompanionConversation: () => {
    streamingAssistantId = null
    set({
      companionMessages: [],
      companionStreaming: false,
      companionContextSummary: '',
      companionWarnings: [],
      companionError: null,
    })
  },
})
