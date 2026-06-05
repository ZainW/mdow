import { dirname } from 'node:path'
import type {
  CompanionProviderId,
  CompanionProviderStatus,
  CompanionSendRequest,
  CompanionSettings,
  CompanionUpdate,
} from '../../shared/types'
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
  let clientKey: string | null = null
  let clientStarted = false
  let activeMessageId = ''

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

  function forwardClientUpdate(update: AcpClientUpdate) {
    if (update.type === 'assistant-delta') {
      emitUpdate({ type: 'assistant-delta', messageId: activeMessageId, text: update.text })
      return
    }
    emitUpdate(update)
  }

  function ensureClient(command: ProviderCommand, cwd: string) {
    const nextKey = JSON.stringify({ ...command, cwd })
    if (client && clientKey === nextKey) {
      return { client, started: clientStarted }
    }

    client?.stop()
    client = createClient({
      ...command,
      cwd,
      onUpdate: forwardClientUpdate,
    })
    clientKey = nextKey
    clientStarted = false
    return { client, started: false }
  }

  return {
    detectProviders: detectedProviders,

    async send(request) {
      const context = await buildContext({
        activePath: request.activePath,
        openFolderPath: request.openFolderPath,
      })
      emitUpdate({ type: 'context', summary: context.summary })
      for (const warning of context.summary.warnings) {
        emitUpdate({ type: 'warning', warning })
      }

      const command = await resolveProvider(request.provider)
      const cwd =
        request.openFolderPath ?? (request.activePath ? dirname(request.activePath) : process.cwd())
      activeMessageId = request.messageId
      const active = ensureClient(command, cwd)

      emitUpdate({ type: 'status', status: 'starting' })
      if (!active.started) {
        await active.client.start()
        clientStarted = true
      }
      emitUpdate({ type: 'status', status: 'streaming' })
      await active.client.sendPrompt(buildCompanionPromptBlocks(request.text, context))
      emitUpdate({ type: 'status', status: 'complete' })
    },

    cancel() {
      client?.cancel()
      emitUpdate({ type: 'status', status: 'cancelled' })
    },

    shutdown() {
      client?.stop()
      client = null
      clientKey = null
      clientStarted = false
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
