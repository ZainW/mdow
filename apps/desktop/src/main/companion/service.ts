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

export class CompanionService {
  private client: AcpClient | null = null
  private activeProvider: CompanionProviderId | null = null
  private lastSources = new Map<string, { path: string; headingId?: string; label: string }>()
  private streamingMessageId: string | null = null

  constructor(private readonly getMainWindow: () => BrowserWindow | null) {}

  private emit(update: CompanionUpdate): void {
    const win = this.getMainWindow()
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
    const preferred =
      providerId ??
      settings.preferredProvider ??
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

    await this.shutdown()

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

  async send(payload: CompanionSendPayload): Promise<void> {
    const cwd = payload.openFolderPath ?? process.cwd()
    const start = await this.startSession(payload.providerId, cwd)
    if (!start.ok || !this.client) {
      this.emit({
        kind: 'error',
        message: start.error ?? 'Companion provider failed to start',
      })
      return
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

    this.streamingMessageId = randomUUID()
    const prompt = formatContextPrompt(packet, payload.text)

    try {
      await this.client.prompt(prompt)
      this.emit({ kind: 'done', messageId: this.streamingMessageId })
    } catch (err) {
      this.emit({
        kind: 'error',
        message: err instanceof Error ? err.message : 'Prompt failed',
      })
    } finally {
      this.streamingMessageId = null
    }
  }

  async cancel(): Promise<void> {
    await this.client?.cancel()
    if (this.streamingMessageId) {
      this.emit({ kind: 'done', messageId: this.streamingMessageId })
      this.streamingMessageId = null
    }
  }

  async shutdown(): Promise<void> {
    const client = this.client
    this.client = null
    this.activeProvider = null
    this.streamingMessageId = null
    if (client) await client.shutdown()
  }

  private handleClientUpdate(update: CompanionUpdate): void {
    if (update.kind === 'delta') {
      const citations = this.extractCitationIds(update.text)
      this.emit(update)
      for (const sourceId of citations) {
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
      return
    }
    this.emit(update)
  }

  private extractCitationIds(text: string): string[] {
    const matches = text.matchAll(/\b(src:[^\s)\]"'`]+)/g)
    const ids: string[] = []
    for (const match of matches) {
      const id = match[1]
      if (this.lastSources.has(id)) ids.push(id)
    }
    return [...new Set(ids)]
  }
}

let companionService: CompanionService | null = null

export function getCompanionService(getMainWindow: () => BrowserWindow | null): CompanionService {
  companionService ??= new CompanionService(getMainWindow)
  return companionService
}
