import { stat } from 'node:fs/promises'
import { dirname } from 'node:path'
import type {
  CompanionProviderId,
  CompanionProviderStatus,
  CompanionSendRequest,
  CompanionSettings,
  CompanionUpdate,
} from '../../shared/types'
import { isPathAllowed } from '../allowed-paths'
import { validateMarkdownPath, validatePath } from '../path-validation'
import { createAcpClient, type AcpClient, type AcpClientUpdate } from './acp-client'
import {
  buildCompanionContext,
  buildCompanionPromptBlocks,
  type BuildCompanionContextInput,
  type CompanionContext,
} from './context-builder'
import { detectCompanionProviders, parseProviderCommand } from './provider-detection'

type ProviderCommand = {
  command: string
  args: string[]
}

type CreateClientOptions = ProviderCommand & {
  cwd: string
  onUpdate: (update: AcpClientUpdate) => void
}

type CompanionServiceDeps = {
  detectProviders: (customCommand: string) => Promise<CompanionProviderStatus[]>
  createClient: (options: CreateClientOptions) => AcpClient
  buildContext: (input: BuildCompanionContextInput) => Promise<CompanionContext>
  getSettings: () => CompanionSettings
  emitUpdate: (update: CompanionUpdate) => void
}

export type CompanionService = {
  detectProviders: () => Promise<CompanionProviderStatus[]>
  send: (request: CompanionSendRequest) => Promise<void>
  cancel: () => void
  shutdown: () => void
}

type CreateDefaultCompanionServiceOptions = {
  getSettings: () => CompanionSettings
  emitUpdate: (update: CompanionUpdate) => void
}

type RequestState = {
  cancelled: boolean
  generation: number
}

const BUILT_IN_COMMANDS: Record<
  Exclude<CompanionProviderId, 'auto' | 'custom'>,
  ProviderCommand
> = {
  opencode: { command: 'opencode', args: ['acp'] },
  codex: { command: 'npx', args: ['--no-install', '@zed-industries/codex-acp'] },
}

export function createCompanionService({
  detectProviders,
  createClient,
  buildContext,
  getSettings,
  emitUpdate,
}: CompanionServiceDeps): CompanionService {
  let client: AcpClient | null = null
  let clientStarted = false
  let inFlight = false
  let activeRequest: RequestState | null = null
  let generation = 0

  async function detectedProviders() {
    return detectProviders(getSettings().customCommand)
  }

  async function resolveProvider(requestProvider: CompanionProviderId): Promise<ProviderCommand> {
    const settings = getSettings()
    const preferredProvider = requestProvider === 'auto' ? settings.provider : requestProvider

    if (preferredProvider === 'auto') {
      const providers = await detectProviders(settings.customCommand)
      const provider = providers.find((candidate) => candidate.status === 'available')
      if (!provider) {
        const message = 'No available companion provider found.'
        emitUpdate({ type: 'error', message })
        throw new Error(message)
      }
      return providerCommand(provider.id, settings.customCommand)
    }

    return providerCommand(preferredProvider, settings.customCommand)
  }

  function forwardClientUpdate(
    requestState: RequestState,
    messageId: string,
    update: AcpClientUpdate,
  ) {
    if (activeRequest !== requestState || isStale(requestState)) {
      return
    }
    if (update.type === 'assistant-delta') {
      emitUpdate({ type: 'assistant-delta', messageId, text: update.text })
      return
    }
    emitUpdate(update)
  }

  function createPromptClient(
    command: ProviderCommand,
    cwd: string,
    requestState: RequestState,
    messageId: string,
  ) {
    client?.stop()
    client = createClient({
      ...command,
      cwd,
      onUpdate: (update) => forwardClientUpdate(requestState, messageId, update),
    })
    clientStarted = false
    return client
  }

  function isStale(requestState: RequestState) {
    return requestState.cancelled || requestState.generation !== generation
  }

  function stopClientIfCurrent(activeClient: AcpClient) {
    activeClient.stop()
    if (client === activeClient) {
      client = null
      clientStarted = false
    }
  }

  return {
    detectProviders: detectedProviders,

    async send(request) {
      if (inFlight) {
        const message = 'A companion response is already in progress.'
        emitUpdate({ type: 'error', message })
        throw new Error(message)
      }

      inFlight = true
      const requestState = { cancelled: false, generation }
      let promptClient: AcpClient | null = null
      activeRequest = requestState
      try {
        const context = await buildContext({
          activePath: request.activePath,
          openFolderPath: request.openFolderPath,
        })
        if (isStale(requestState)) {
          return
        }
        emitUpdate({ type: 'context', summary: context.summary })
        for (const warning of context.summary.warnings) {
          emitUpdate({ type: 'warning', warning })
        }
        if (isStale(requestState)) {
          return
        }

        const command = await resolveProvider(request.provider)
        if (isStale(requestState)) {
          return
        }
        const cwd = await safeCompanionCwd(request)
        if (isStale(requestState)) {
          return
        }
        promptClient = createPromptClient(command, cwd, requestState, request.messageId)
        if (isStale(requestState)) {
          return
        }

        emitUpdate({ type: 'status', status: 'starting' })
        if (!clientStarted) {
          await promptClient.start()
          clientStarted = true
        }
        if (isStale(requestState)) {
          return
        }
        emitUpdate({ type: 'status', status: 'streaming' })
        if (isStale(requestState)) {
          return
        }
        await promptClient.sendPrompt(buildCompanionPromptBlocks(request.text, context))
        if (!isStale(requestState)) {
          emitUpdate({ type: 'status', status: 'complete' })
        }
      } finally {
        if (promptClient) {
          stopClientIfCurrent(promptClient)
        }
        if (activeRequest === requestState) {
          activeRequest = null
          inFlight = false
        }
      }
    },

    cancel() {
      if (!activeRequest || activeRequest.cancelled) {
        return
      }
      activeRequest.cancelled = true
      client?.cancel()
      emitUpdate({ type: 'status', status: 'cancelled' })
    },

    shutdown() {
      generation += 1
      if (activeRequest) {
        activeRequest.cancelled = true
      }
      client?.stop()
      client = null
      clientStarted = false
      activeRequest = null
      inFlight = false
    },
  }
}

export function createDefaultCompanionService(
  options: CreateDefaultCompanionServiceOptions,
): CompanionService {
  return createCompanionService({
    detectProviders: detectCompanionProviders,
    createClient: ({ command, args, cwd, onUpdate }) =>
      createAcpClient({ command, args, cwd, onUpdate }),
    buildContext: buildCompanionContext,
    getSettings: options.getSettings,
    emitUpdate: options.emitUpdate,
  })
}

function providerCommand(provider: Exclude<CompanionProviderId, 'auto'>, customCommand: string) {
  if (provider === 'custom') {
    const parsed = parseProviderCommand(customCommand)
    if (!parsed.command) {
      throw new Error('Custom companion provider command is empty.')
    }
    return parsed
  }
  return BUILT_IN_COMMANDS[provider]
}

async function safeCompanionCwd(request: CompanionSendRequest): Promise<string> {
  if (request.openFolderPath) {
    const folderPath = await safeDirectoryPath(request.openFolderPath)
    if (folderPath) {
      return folderPath
    }
  }

  if (request.activePath) {
    const filePath = await safeMarkdownFilePath(request.activePath)
    if (filePath) {
      return dirname(filePath)
    }
  }

  return process.cwd()
}

async function safeDirectoryPath(path: string): Promise<string | null> {
  try {
    const resolved = validatePath(path)
    if (!isPathAllowed(resolved)) {
      return null
    }
    const stats = await stat(resolved)
    return stats.isDirectory() ? resolved : null
  } catch {
    return null
  }
}

async function safeMarkdownFilePath(path: string): Promise<string | null> {
  try {
    const resolved = validateMarkdownPath(path)
    if (!isPathAllowed(resolved)) {
      return null
    }
    const stats = await stat(resolved)
    return stats.isFile() ? resolved : null
  } catch {
    return null
  }
}
