import { useEffect, useRef } from 'react'
import { extractCompanionCitations } from '../lib/companion-citations'
import { selectActiveTab, useAppStore } from '../store/app-store'

type ActiveCompanionRequest = {
  messageId: string
  cancelled: boolean
}

export function useCompanionController() {
  const activeRequestRef = useRef<ActiveCompanionRequest | null>(null)

  function markActiveRequestCancelled() {
    const activeRequest = activeRequestRef.current
    if (!activeRequest) return

    activeRequest.cancelled = true
    const store = useAppStore.getState()
    const message = store.companionMessages.find((item) => item.id === activeRequest.messageId)
    if (message) {
      store.updateCompanionMessage({ ...message, status: 'error' })
    }
    store.setCompanionStreaming(false)
  }

  useEffect(() => {
    let disposed = false
    const unsubscribe = window.api.onCompanionUpdate((update) => {
      const store = useAppStore.getState()

      if (update.type === 'status') {
        if (update.status === 'starting' || update.status === 'streaming') {
          store.setCompanionStreaming(true)
        } else if (update.status === 'complete' || update.status === 'cancelled') {
          if (update.status === 'cancelled') {
            markActiveRequestCancelled()
          }
          store.setCompanionStreaming(false)
        }
        return
      }

      if (update.type === 'context') {
        store.setCompanionContext(update.summary)
        return
      }

      if (update.type === 'assistant-delta') {
        store.appendCompanionAssistantDelta(update.messageId, update.text)
        return
      }

      if (update.type === 'error') {
        store.setCompanionError(update.message)
        store.setCompanionStreaming(false)
        return
      }

      if (update.type === 'warning') {
        store.setCompanionError(update.warning.message)
        return
      }

      if (update.type === 'tool-refused') {
        store.setCompanionError(`Tool refused: ${update.title}`)
      }
    })

    void window.api
      .getCompanionSettings()
      .then((settings) => {
        if (!disposed) {
          useAppStore.setState({
            companionProvider: settings.provider,
            companionCustomCommand: settings.customCommand,
          })
        }
      })
      .catch((error: unknown) => {
        if (!disposed) {
          useAppStore.getState().setCompanionError(errorMessage(error))
        }
      })

    void window.api
      .detectCompanionProviders()
      .then((providers) => {
        if (!disposed) {
          useAppStore.getState().setCompanionProviders(providers)
        }
      })
      .catch((error: unknown) => {
        if (!disposed) {
          useAppStore.getState().setCompanionError(errorMessage(error))
        }
      })

    return () => {
      disposed = true
      if (activeRequestRef.current) {
        markActiveRequestCancelled()
        void window.api.cancelCompanionMessage().catch((error: unknown) => {
          useAppStore.getState().setCompanionError(errorMessage(error))
        })
      }
      unsubscribe()
    }
  }, [])

  return {
    send: async (text: string) => {
      const trimmed = text.trim()
      if (!trimmed) return

      const store = useAppStore.getState()
      if (activeRequestRef.current || store.companionStreaming) return

      const activeTab = selectActiveTab(store)
      const userMessage = store.appendCompanionMessage('user', trimmed)
      const assistantMessage = store.appendCompanionMessage('assistant', '', 'streaming')
      const activeRequest = { messageId: assistantMessage.id, cancelled: false }
      activeRequestRef.current = activeRequest
      store.setCompanionError(null)
      store.setCompanionStreaming(true)

      try {
        await window.api.sendCompanionMessage({
          messageId: assistantMessage.id,
          text: userMessage.content,
          provider: store.companionProvider,
          activePath: activeTab?.path ?? null,
          openFolderPath: store.openFolderPath,
        })

        const latest = useAppStore.getState()
        const message = latest.companionMessages.find((item) => item.id === assistantMessage.id)
        if (message) {
          if (activeRequest.cancelled) {
            latest.updateCompanionMessage({ ...message, status: 'error' })
            return
          }
          const citations = extractCompanionCitations(
            message.content,
            latest.companionContext?.sources ?? [],
          )
          latest.updateCompanionMessage({
            ...message,
            content: citations.text,
            citations: citations.citations,
            status: 'complete',
          })
        }
      } catch (error) {
        const latest = useAppStore.getState()
        const message = latest.companionMessages.find((item) => item.id === assistantMessage.id)
        if (message) {
          latest.updateCompanionMessage({ ...message, status: 'error' })
        }
        if (!activeRequest.cancelled) {
          latest.setCompanionError(errorMessage(error))
        }
      } finally {
        if (activeRequestRef.current === activeRequest) {
          activeRequestRef.current = null
        }
        useAppStore.getState().setCompanionStreaming(false)
      }
    },
    cancel: async () => {
      markActiveRequestCancelled()
      try {
        await window.api.cancelCompanionMessage()
      } finally {
        useAppStore.getState().setCompanionStreaming(false)
      }
    },
  }
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}
