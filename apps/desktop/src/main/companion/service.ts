import type { BrowserWindow } from 'electron'
import { randomUUID } from 'crypto'
import { IPC } from '../../shared/types'
import type {
  CompanionProviderId,
  CompanionSendPayload,
  CompanionSettings,
  CompanionStartResult,
  CompanionUpdate,
} from '../../shared/types'
import { getCompanionSettings, saveCompanionSettings } from '../store'
import { AcpClient } from './acp-client'
import { buildCompanionContext, formatContextPrompt } from './context-builder'
import { detectCompanionProviders, resolveProviderCommand } from './provider-detection'

interface CitationStreamResult {
  text: string
  citationIds: string[]
}

export class CitationStream {
  private buffer = ''
  private readonly sourceIds: string[]
  private readonly emittedIds = new Set<string>()

  constructor(sourceIds: Iterable<string>) {
    this.sourceIds = [...new Set(sourceIds)].toSorted((a, b) => b.length - a.length)
  }

  consume(text: string): CitationStreamResult {
    this.buffer += text
    const citationIds = this.removeKnownSourceIds()
    this.buffer = this.removeEmptyCitationWrappers(this.buffer)

    const carryLength = this.getCarryLength()
    const visibleLength = this.buffer.length - carryLength
    const visible = this.buffer.slice(0, visibleLength)
    this.buffer = this.buffer.slice(visibleLength)
    return { text: visible, citationIds }
  }

  flush(): CitationStreamResult {
    const citationIds = this.removeKnownSourceIds()
    const text = this.removeEmptyCitationWrappers(this.buffer)
    this.buffer = ''
    return { text, citationIds }
  }

  private removeKnownSourceIds(): string[] {
    const citationIds: string[] = []
    for (const sourceId of this.sourceIds) {
      const withoutSourceId = this.buffer.replaceAll(sourceId, '')
      if (withoutSourceId === this.buffer) continue
      this.buffer = withoutSourceId
      if (!this.emittedIds.has(sourceId)) {
        this.emittedIds.add(sourceId)
        citationIds.push(sourceId)
      }
    }
    return citationIds
  }

  private removeEmptyCitationWrappers(text: string): string {
    return text.replace(/\(\s*\)/g, '').replace(/\[\s*\]/g, '')
  }

  private getCarryLength(): number {
    let carryLength = 0
    for (const sourceId of this.sourceIds) {
      const maxPrefixLength = Math.min(sourceId.length - 1, this.buffer.length)
      for (let length = maxPrefixLength; length > carryLength; length -= 1) {
        if (this.buffer.endsWith(sourceId.slice(0, length))) {
          carryLength = length
          break
        }
      }
    }

    const carryStart = this.buffer.length - carryLength
    if (carryLength > 0 && /[([]/.test(this.buffer.charAt(carryStart - 1))) {
      carryLength += 1
    } else if (carryLength === 0 && /[([]$/.test(this.buffer)) {
      carryLength = 1
    }
    return carryLength
  }
}

export class CompanionService {
  private client: AcpClient | null = null
  private activeProvider: CompanionProviderId | null = null
  private lastSources = new Map<string, { path: string; headingId?: string; label: string }>()
  private streamingMessageId: string | null = null
  private citationStream = new CitationStream([])
  private activeRequestToken: symbol | null = null
  private activeWindow: BrowserWindow | null = null

  constructor(private readonly getMainWindow: () => BrowserWindow | null) {}

  private emit(update: CompanionUpdate, targetWindow?: BrowserWindow | null): void {
    const win = targetWindow ?? this.activeWindow ?? this.getMainWindow()
    if (!win || win.isDestroyed()) return
    win.webContents.send(IPC.COMPANION_UPDATE, update)
  }

  async detectProviders() {
    return detectCompanionProviders()
  }

  getSettings(): CompanionSettings {
    return getCompanionSettings()
  }

  saveSettings(settings: Partial<CompanionSettings>): void {
    saveCompanionSettings(settings)
  }

  async startSession(
    providerId?: CompanionProviderId,
    cwd = process.cwd(),
  ): Promise<CompanionStartResult> {
    const settings = getCompanionSettings()
    const providers = await detectCompanionProviders()
    const savedProviderAvailable = providers.some(
      (provider) =>
        provider.id === settings.preferredProvider && provider.availability === 'available',
    )
    const preferred =
      providerId ??
      (savedProviderAvailable ? settings.preferredProvider : null) ??
      providers.find((p) => p.availability === 'available')?.id ??
      null

    if (!preferred) {
      return { ok: false, providerId: null, error: 'No companion provider available' }
    }

    const command = resolveProviderCommand(preferred, settings.customCommand)
    if (!command) {
      return { ok: false, providerId: preferred, error: 'Provider command is not configured' }
    }

    if (this.client && this.activeProvider === preferred && this.client.getSessionId()) {
      return { ok: true, providerId: preferred }
    }

    await this.shutdownClient()

    const client = new AcpClient({
      command: command.command,
      args: command.args,
      cwd,
      onUpdate: (update) => this.handleClientUpdate(update),
    })

    try {
      await client.start()
      await client.createSession(cwd)
      this.client = client
      this.activeProvider = preferred
      return { ok: true, providerId: preferred }
    } catch (err) {
      await client.shutdown()
      this.client = null
      this.activeProvider = null
      return {
        ok: false,
        providerId: preferred,
        error: err instanceof Error ? err.message : 'Failed to start provider',
      }
    }
  }

  async send(
    payload: CompanionSendPayload,
    targetWindow: BrowserWindow | null = this.getMainWindow(),
  ): Promise<void> {
    if (this.activeRequestToken) {
      this.emit(
        {
          kind: 'warning',
          message: 'Wait for the current response or cancel it first.',
        },
        targetWindow,
      )
      return
    }

    const requestToken = Symbol('companion-request')
    this.activeRequestToken = requestToken
    this.activeWindow = targetWindow
    let messageId: string | null = null

    try {
      const cwd = payload.openFolderPath ?? process.cwd()
      const start = await this.startSession(payload.providerId, cwd)
      if (!start.ok || !this.client) {
        throw new Error(start.error ?? 'Companion provider failed to start')
      }

      const packet = await buildCompanionContext({
        activePath: payload.activePath,
        openFolderPath: payload.openFolderPath,
        tags: payload.tags,
      })

      this.lastSources.clear()
      for (const source of packet.sources) {
        this.lastSources.set(source.sourceId, {
          path: source.path,
          headingId: source.headingId,
          label: source.path.split(/[/\\]/).pop() ?? source.path,
        })
      }

      this.emit({
        kind: 'context',
        summary: packet.summary,
        warnings: packet.warnings,
      })

      this.citationStream = new CitationStream(this.lastSources.keys())
      messageId = randomUUID()
      this.streamingMessageId = messageId
      const prompt = formatContextPrompt(packet, payload.text)
      await this.client.prompt(prompt)
      if (this.activeRequestToken !== requestToken || this.streamingMessageId !== messageId) {
        return
      }
      this.emitCitationResult(this.citationStream.flush())
      this.emit({ kind: 'done', messageId })
    } catch (err) {
      if (this.activeRequestToken === requestToken) {
        this.emit({
          kind: 'error',
          message: err instanceof Error ? err.message : 'Prompt failed',
        })
      }
    } finally {
      if (messageId && this.streamingMessageId === messageId) {
        this.streamingMessageId = null
      }
      if (this.activeRequestToken === requestToken) {
        this.activeRequestToken = null
        this.activeWindow = null
      }
    }
  }

  async cancel(): Promise<void> {
    const messageId = this.streamingMessageId
    const targetWindow = this.activeWindow
    const client = this.client
    this.streamingMessageId = null
    this.activeRequestToken = null
    this.citationStream = new CitationStream([])
    this.client = null
    this.activeProvider = null
    if (client) {
      try {
        await client.cancel()
      } catch {
        // The process may exit between the user's cancel action and the notification write.
      } finally {
        await client.shutdown()
      }
    }
    if (messageId) {
      this.emit({ kind: 'cancelled', messageId }, targetWindow)
    }
    this.activeWindow = null
  }

  async shutdown(): Promise<void> {
    this.activeRequestToken = null
    this.activeWindow = null
    await this.shutdownClient()
  }

  private async shutdownClient(): Promise<void> {
    const client = this.client
    this.client = null
    this.activeProvider = null
    this.streamingMessageId = null
    this.citationStream = new CitationStream([])
    if (client) await client.shutdown()
  }

  handleClientUpdate(update: CompanionUpdate): void {
    if (!this.activeRequestToken) return
    if (update.kind === 'delta') {
      this.emitCitationResult(this.citationStream.consume(update.text))
      return
    }
    this.emit(update)
  }

  private emitCitationResult(result: CitationStreamResult): void {
    if (result.text) {
      this.emit({ kind: 'delta', text: result.text })
    }
    for (const sourceId of result.citationIds) {
      const source = this.lastSources.get(sourceId)
      if (!source) continue
      this.emit({
        kind: 'citation',
        citation: {
          sourceId,
          path: source.path,
          headingId: source.headingId,
          label: source.label,
        },
      })
    }
  }
}

let companionService: CompanionService | null = null

export function getCompanionService(getMainWindow: () => BrowserWindow | null): CompanionService {
  companionService ??= new CompanionService(getMainWindow)
  return companionService
}
