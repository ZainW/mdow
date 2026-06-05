import { useEffect } from 'react'
import { extractCompanionCitations } from '../lib/companion-citations'
import { selectActiveTab, useAppStore } from '../store/app-store'

type ActiveCompanionRequest = {
  messageId: string
  cancelled: boolean
}

let activeRequest: ActiveCompanionRequest | null = null
let mountedControllers = 0
let unsubscribeCompanionUpdates: (() => void) | null = null
let controllerGeneration = 0

function markActiveRequestCancelled() {
  if (!activeRequest) return

  activeRequest.cancelled = true
  const store = useAppStore.getState()
  const message = store.companionMessages.find((item) => item.id === activeRequest!.messageId)
  if (message) {
    store.updateCompanionMessage({ ...message, status: 'error' })
  }
  store.setCompanionStreaming(false)
}

export function useCompanionController() {
  useEffect(() => {
    mountedControllers += 1
    if (!unsubscribeCompanionUpdates) {
      controllerGeneration += 1
      const generation = controllerGeneration

      unsubscribeCompanionUpdates = window.api.onCompanionUpdate((update) => {
        const store = useAppStore.getState()

        if (update.type === 'status') {
          if (update.status === 'starting' || update.status === 'streaming') {
            store.setCompanionStreaming(true)
          } else if (
            (update.status === 'complete' || update.status === 'cancelled') &&
            !activeRequest
          ) {
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
          if (mountedControllers > 0 && generation === controllerGeneration) {
            useAppStore.setState({
              companionProvider: settings.provider,
              companionCustomCommand: settings.customCommand,
            })
          }
        })
        .catch((error: unknown) => {
          if (mountedControllers > 0 && generation === controllerGeneration) {
            useAppStore.getState().setCompanionError(errorMessage(error))
          }
        })

      void window.api
        .detectCompanionProviders()
        .then((providers) => {
          if (mountedControllers > 0 && generation === controllerGeneration) {
            useAppStore.getState().setCompanionProviders(providers)
          }
        })
        .catch((error: unknown) => {
          if (mountedControllers > 0 && generation === controllerGeneration) {
            useAppStore.getState().setCompanionError(errorMessage(error))
          }
        })
    }

    return () => {
      mountedControllers = Math.max(0, mountedControllers - 1)
      if (mountedControllers === 0) {
        markActiveRequestCancelled()
        if (activeRequest) {
          void window.api.cancelCompanionMessage().catch((error: unknown) => {
            useAppStore.getState().setCompanionError(errorMessage(error))
          })
          activeRequest = null
        }
        unsubscribeCompanionUpdates?.()
        unsubscribeCompanionUpdates = null
        controllerGeneration += 1
      }
    }
  }, [])

  return {
    send: async (text: string) => {
      const trimmed = text.trim()
      if (!trimmed) return

      const store = useAppStore.getState()
      if (activeRequest || store.companionStreaming) return

      const activeTab = selectActiveTab(store)
      const userMessage = store.appendCompanionMessage('user', trimmed)
      const assistantMessage = store.appendCompanionMessage('assistant', '', 'streaming')
      const request = { messageId: assistantMessage.id, cancelled: false }
      activeRequest = request
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
          if (request.cancelled) {
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
        if (!request.cancelled) {
          latest.setCompanionError(errorMessage(error))
        }
      } finally {
        if (activeRequest === request) {
          activeRequest = null
          useAppStore.getState().setCompanionStreaming(false)
        }
      }
    },
    cancel: async () => {
      const request = activeRequest
      markActiveRequestCancelled()
      if (activeRequest === request) {
        activeRequest = null
      }
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
