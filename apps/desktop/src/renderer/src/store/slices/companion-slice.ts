import type { StateCreator } from 'zustand'
import type {
  CompanionCitation,
  CompanionContextTag,
  CompanionMessage,
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
      companionMessages: [...state.companionMessages, message],
      companionError: message.role === 'user' ? null : state.companionError,
    })),
  applyCompanionUpdate: (update) => {
    switch (update.kind) {
      case 'delta':
        set((state) => {
          if (!streamingAssistantId) {
            streamingAssistantId = crypto.randomUUID()
            return {
              companionStreaming: true,
              companionMessages: [
                ...state.companionMessages,
                {
                  id: streamingAssistantId,
                  role: 'assistant',
                  content: update.text,
                  status: 'streaming',
                  citations: [],
                },
              ],
            }
          }
          return {
            companionStreaming: true,
            companionMessages: state.companionMessages.map((m) =>
              m.id === streamingAssistantId
                ? { ...m, content: m.content + update.text, status: 'streaming' }
                : m,
            ),
          }
        })
        break
      case 'status':
        break
      case 'citation':
        set((state) => {
          if (!streamingAssistantId) return state
          return {
            companionMessages: state.companionMessages.map((m) => {
              if (m.id !== streamingAssistantId) return m
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
              ? { ...m, status: 'complete' }
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
