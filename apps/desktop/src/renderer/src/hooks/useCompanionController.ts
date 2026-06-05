import { useEffect } from 'react'
import { extractCompanionCitations } from '../lib/companion-citations'
import { selectActiveTab, useAppStore } from '../store/app-store'

export function useCompanionController() {
  useEffect(() => {
    let disposed = false
    const unsubscribe = window.api.onCompanionUpdate((update) => {
      const store = useAppStore.getState()

      if (update.type === 'status') {
        if (update.status === 'starting' || update.status === 'streaming') {
          store.setCompanionStreaming(true)
        } else if (update.status === 'complete' || update.status === 'cancelled') {
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
      unsubscribe()
    }
  }, [])

  return {
    send: async (text: string) => {
      const trimmed = text.trim()
      if (!trimmed) return

      const store = useAppStore.getState()
      const activeTab = selectActiveTab(store)
      const userMessage = store.appendCompanionMessage('user', trimmed)
      const assistantMessage = store.appendCompanionMessage('assistant', '', 'streaming')
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
        latest.setCompanionError(errorMessage(error))
      } finally {
        useAppStore.getState().setCompanionStreaming(false)
      }
    },
    cancel: async () => {
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
