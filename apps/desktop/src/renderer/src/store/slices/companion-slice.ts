import type { StateCreator } from 'zustand'
import type {
  CompanionContextSummary,
  CompanionMessage,
  CompanionMessageRole,
  CompanionMessageStatus,
  CompanionProviderId,
  CompanionProviderStatus,
} from '../../../../shared/types'

type CompanionState = {
  companionOpen: boolean
  companionFullscreen: boolean
  companionStreaming: boolean
  companionProvider: CompanionProviderId
  companionCustomCommand: string
  companionProviders: CompanionProviderStatus[]
  companionMessages: CompanionMessage[]
  companionContext: CompanionContextSummary | null
  companionError: string | null
}

const initialCompanionState: CompanionState = {
  companionOpen: false,
  companionFullscreen: false,
  companionStreaming: false,
  companionProvider: 'auto',
  companionCustomCommand: '',
  companionProviders: [],
  companionMessages: [],
  companionContext: null,
  companionError: null,
}

export interface CompanionSlice extends CompanionState {
  setCompanionOpen: (open: boolean) => void
  setCompanionFullscreen: (fullscreen: boolean) => void
  setCompanionStreaming: (streaming: boolean) => void
  setCompanionProvider: (provider: CompanionProviderId) => void
  setCompanionCustomCommand: (command: string) => void
  setCompanionProviders: (providers: CompanionProviderStatus[]) => void
  setCompanionMessages: (messages: CompanionMessage[]) => void
  setCompanionContext: (context: CompanionContextSummary | null) => void
  setCompanionError: (error: string | null) => void
  appendCompanionMessage: (
    role: CompanionMessageRole,
    content: string,
    status?: CompanionMessageStatus,
  ) => CompanionMessage
  appendCompanionAssistantDelta: (messageId: string, delta: string) => void
  updateCompanionMessage: (message: CompanionMessage) => void
  resetCompanion: () => void
}

export const createCompanionSlice: StateCreator<CompanionSlice, [], [], CompanionSlice> = (set) => ({
  ...initialCompanionState,
  setCompanionOpen: (open) => set({ companionOpen: open }),
  setCompanionFullscreen: (fullscreen) => set({ companionFullscreen: fullscreen }),
  setCompanionStreaming: (streaming) => set({ companionStreaming: streaming }),
  setCompanionProvider: (provider) => {
    if (typeof window !== 'undefined' && window.api) {
      void window.api.saveAppState({ companionProvider: provider })
    }
    set({ companionProvider: provider })
  },
  setCompanionCustomCommand: (command) => {
    if (typeof window !== 'undefined' && window.api) {
      void window.api.saveAppState({ companionCustomCommand: command })
    }
    set({ companionCustomCommand: command })
  },
  setCompanionProviders: (providers) => set({ companionProviders: providers }),
  setCompanionMessages: (messages) => set({ companionMessages: messages }),
  setCompanionContext: (context) => set({ companionContext: context }),
  setCompanionError: (error) => set({ companionError: error }),
  appendCompanionMessage: (role, content, status = 'complete') => {
    const message: CompanionMessage = {
      id: crypto.randomUUID(),
      role,
      content,
      status,
      citations: [],
      createdAt: Date.now(),
    }
    set((state) => ({ companionMessages: [...state.companionMessages, message] }))
    return message
  },
  appendCompanionAssistantDelta: (messageId, delta) =>
    set((state) => ({
      companionMessages: state.companionMessages.map((message) =>
        message.id === messageId ? { ...message, content: message.content + delta } : message,
      ),
    })),
  updateCompanionMessage: (message) =>
    set((state) => ({
      companionMessages: state.companionMessages.map((item) =>
        item.id === message.id ? message : item,
      ),
    })),
  resetCompanion: () => set({ ...initialCompanionState }),
})
