import type { BrowserWindow } from 'electron'
import { randomUUID } from 'crypto'
import { homedir } from 'os'
import { dirname, relative, sep } from 'path'
import { IPC } from '../../shared/types'
import type {
  CompanionModelOption,
  CompanionModelState,
  CompanionPermissionDecision,
  CompanionRuntimeStatus,
  CompanionSendPayload,
  CompanionUpdate,
} from '../../shared/types'
import { getCompanionSettings, saveCompanionSettings } from '../store'
import { detectOpencode, type ResolvedOpencode } from './opencode-binary'
import {
  OpencodeClient,
  type OpencodeEvent,
  type OpencodeModelRef,
  type OpencodePermissionRule,
  type OpencodeProviderInfo,
} from './opencode-client'
import { OpencodeTurnTranslator, type TurnFinish } from './opencode-events'
import { startOpencodeServer, type OpencodeServerHandle } from './opencode-server'

export const COMPANION_INSTRUCTIONS = `You are the writing companion inside Mdow, a calm reader for Markdown and HTML documents. The user is reading the documents in this folder and wants help understanding and improving them.

- Answer questions about the documents. Read them with your tools instead of guessing, and mention documents by their path relative to the folder.
- When the user asks for a change, make it with your edit or write tool right away. Mdow shows the user every change as a diff to approve or reject before it is written, so do not ask for confirmation first.
- Keep edits focused. Preserve each document's voice, structure and formatting, whether it is Markdown or HTML.
- Do not run shell commands, reach outside this folder, or create new files unless the user asks for one.
- Reply concisely in Markdown.`

// Shell commands and other folders are off limits; edits always come back to the user for review.
const SESSION_PERMISSIONS: OpencodePermissionRule[] = [
  { action: 'shell', resource: '*', effect: 'deny' },
  { action: 'bash', resource: '*', effect: 'deny' },
  { action: 'edit', resource: '*', effect: 'ask' },
]

const MODEL_WARMUP_ATTEMPTS = 8
const MODEL_WARMUP_DELAY_MS = 500

interface Runtime {
  binary: ResolvedOpencode
  server: OpencodeServerHandle
  client: OpencodeClient
}

interface ActiveSession {
  id: string
  directory: string
  translator: OpencodeTurnTranslator
  events: AbortController
}

interface ActiveTurn {
  messageId: string
  window: BrowserWindow | null
  /** When the prompt was sent, to tell this turn's finish from an earlier run's. */
  promptedAt: number
  /**
   * Set once OpenCode starts the run for this prompt. Until then, events still arriving from
   * a run the user stopped are ignored, so a late "interrupted" cannot end the new turn.
   */
  started: boolean
}

const EVENT_STREAM_TIMEOUT_MS = 10_000

export function parseModelValue(value: string): OpencodeModelRef | null {
  const slash = value.indexOf('/')
  if (slash <= 0 || slash === value.length - 1) return null
  return { providerID: value.slice(0, slash), id: value.slice(slash + 1) }
}

export function sessionDirectoryFor(payload: CompanionSendPayload): string {
  if (payload.openFolderPath) return payload.openFolderPath
  if (payload.activePath) return dirname(payload.activePath)
  return homedir()
}

function displayPath(directory: string, path: string): string {
  const rel = relative(directory, path)
  return rel && !rel.startsWith('..') && !rel.startsWith(sep) ? rel : path
}

/** Prefixes the user's message with what they are looking at in Mdow. */
export function composePrompt(payload: CompanionSendPayload, directory: string): string {
  const lines: string[] = []
  if (payload.activePath) lines.push(`Viewing: ${displayPath(directory, payload.activePath)}`)
  const attached = payload.tags.map((tag) => displayPath(directory, tag.path))
  if (attached.length) lines.push(`Attached: ${attached.join(', ')}`)
  if (lines.length === 0) return payload.text
  return `<mdow-context>\n${lines.join('\n')}\n</mdow-context>\n\n${payload.text}`
}

export class CompanionService {
  private runtime: Runtime | null = null
  private runtimeStarting: Promise<Runtime> | null = null
  private session: ActiveSession | null = null
  private turn: ActiveTurn | null = null
  private readonly pendingPermissions = new Set<string>()
  private modelOptions: CompanionModelOption[] = []

  constructor(private readonly getMainWindow: () => BrowserWindow | null) {}

  private emit(update: CompanionUpdate): void {
    const win = this.turn?.window ?? this.getMainWindow()
    if (!win || win.isDestroyed()) return
    win.webContents.send(IPC.COMPANION_UPDATE, update)
  }

  async getStatus(): Promise<CompanionRuntimeStatus> {
    if (this.runtime) return { availability: 'available', version: this.runtime.binary.version }
    return (await detectOpencode()).status
  }

  async start(): Promise<CompanionRuntimeStatus> {
    try {
      const runtime = await this.ensureRuntime()
      return { availability: 'available', version: runtime.binary.version }
    } catch (error) {
      const status = await this.getStatus()
      if (status.availability !== 'available') return status
      return {
        availability: 'failed',
        version: status.version,
        detail: error instanceof Error ? error.message : 'OpenCode failed to start.',
      }
    }
  }

  private ensureRuntime(): Promise<Runtime> {
    if (this.runtime) return Promise.resolve(this.runtime)
    this.runtimeStarting ??= (async () => {
      const { binary, status } = await detectOpencode()
      if (!binary) throw new Error(status.detail ?? 'OpenCode is not available.')
      const server = await startOpencodeServer(binary.path)
      const runtime: Runtime = {
        binary,
        server,
        client: new OpencodeClient(server.url, server.password),
      }
      void server.exited.then(() => this.handleServerExit(runtime))
      this.runtime = runtime
      return runtime
    })().finally(() => {
      this.runtimeStarting = null
    })
    return this.runtimeStarting
  }

  private handleServerExit(runtime: Runtime): void {
    if (this.runtime !== runtime) return
    this.runtime = null
    this.closeSession()
    if (this.turn) {
      this.emit({
        kind: 'error',
        message: 'OpenCode stopped unexpectedly. Send again to restart it.',
      })
      this.turn = null
    }
  }

  async getModels(directory: string = homedir()): Promise<CompanionModelState> {
    let runtime: Runtime
    try {
      runtime = await this.ensureRuntime()
    } catch (error) {
      return {
        options: [],
        currentValue: null,
        stale: true,
        unavailableReason: error instanceof Error ? error.message : 'OpenCode is not available.',
      }
    }

    // A fresh server loads the model catalog in the background, so the first answers can be empty.
    let models = await runtime.client.listModels(directory)
    // oxlint-disable-next-line eslint/no-await-in-loop -- polls until the catalog finishes loading
    for (let attempt = 1; models.length === 0 && attempt < MODEL_WARMUP_ATTEMPTS; attempt += 1) {
      await new Promise((resolve) => setTimeout(resolve, MODEL_WARMUP_DELAY_MS))
      models = await runtime.client.listModels(directory)
    }
    const providers = await runtime.client
      .listProviders(directory)
      .catch((): OpencodeProviderInfo[] => [])
    const providerNames = new Map(
      providers.map((provider) => [provider.id, provider.name ?? provider.id] as const),
    )

    this.modelOptions = models
      .filter((model) => model.status !== 'deprecated')
      .map((model) => ({
        value: `${model.providerID}/${model.id}`,
        name: model.name ?? model.id,
        providerId: model.providerID,
        providerName: providerNames.get(model.providerID) ?? model.providerID,
      }))
      .toSorted(
        (a, b) => a.providerName.localeCompare(b.providerName) || a.name.localeCompare(b.name),
      )

    const saved = getCompanionSettings().lastModel
    let currentValue = saved && this.hasModel(saved) ? saved : null
    if (!currentValue) {
      const fallback = await runtime.client.defaultModel(directory).catch(() => null)
      const value = fallback ? `${fallback.providerID}/${fallback.id}` : null
      currentValue = value && this.hasModel(value) ? value : null
    }

    return {
      options: this.modelOptions,
      currentValue,
      stale: false,
      ...(this.modelOptions.length === 0
        ? { unavailableReason: 'No models yet. Run `opencode auth login` to connect a provider.' }
        : {}),
    }
  }

  private hasModel(value: string): boolean {
    return this.modelOptions.some((option) => option.value === value)
  }

  async setModel(value: string): Promise<CompanionModelState> {
    const ref = parseModelValue(value)
    if (!ref || !this.hasModel(value)) throw new Error(`Model is not available: ${value}`)
    saveCompanionSettings({ lastModel: value })
    if (this.session && this.runtime) {
      await this.runtime.client.switchModel(this.session.id, ref)
    }
    return { options: this.modelOptions, currentValue: value, stale: false }
  }

  private async ensureSession(runtime: Runtime, directory: string): Promise<ActiveSession> {
    if (this.session?.directory === directory) return this.session
    this.closeSession()

    const created = await runtime.client.createSession({
      directory,
      model: await this.sessionModel(directory),
      permissions: SESSION_PERMISSIONS,
    })
    try {
      await runtime.client.setInstructions(created.id, 'mdow', COMPANION_INSTRUCTIONS)
    } catch {
      // Older 2.x servers may lack session instructions; the permission rules still apply.
    }

    const session: ActiveSession = {
      id: created.id,
      directory,
      translator: new OpencodeTurnTranslator(created.id, directory),
      events: new AbortController(),
    }
    this.session = session

    // Prompting before the stream is open would lose the start of the response, so wait for it.
    let markOpen = () => undefined as void
    let failOpen = (_error: Error) => undefined as void
    const opened = new Promise<void>((resolve, reject) => {
      markOpen = resolve
      failOpen = reject
    })
    const timer = setTimeout(
      () => failOpen(new Error('OpenCode did not open its event stream.')),
      EVENT_STREAM_TIMEOUT_MS,
    )
    void runtime.client
      .subscribe(
        directory,
        (event) => this.handleEvent(session, event),
        session.events.signal,
        (reconnected) => {
          markOpen()
          if (reconnected) void this.reconcileTurn(session)
        },
      )
      .catch((error: unknown) => {
        const failure = error instanceof Error ? error : new Error('Lost connection to OpenCode.')
        failOpen(failure)
        if (this.session !== session || !this.turn) return
        this.emit({ kind: 'error', message: failure.message })
        this.turn = null
      })
    try {
      await opened
    } catch (error) {
      if (this.session === session) this.closeSession()
      throw error
    } finally {
      clearTimeout(timer)
    }
    return session
  }

  /** The saved model when OpenCode still offers it, otherwise OpenCode's own default. */
  private async sessionModel(directory: string): Promise<OpencodeModelRef | null> {
    const saved = getCompanionSettings().lastModel
    if (!saved) return null
    if (this.modelOptions.length === 0) await this.getModels(directory)
    return this.hasModel(saved) ? parseModelValue(saved) : null
  }

  /**
   * Events sent while the stream was reconnecting are not replayed. If the turn finished in
   * that gap, read how it ended from the session instead of waiting forever.
   */
  private async reconcileTurn(session: ActiveSession): Promise<void> {
    const turn = this.turn
    if (!turn || !this.runtime || this.session !== session) return
    let info
    try {
      info = await this.runtime.client.getSession(session.id)
    } catch {
      return
    }
    if (this.turn !== turn || !info.outcome || !info.time.idle) return
    if (info.time.idle < turn.promptedAt) return
    turn.started = true
    this.finishTurn(
      info.outcome === 'succeeded' ? { outcome: 'succeeded' } : { outcome: info.outcome },
    )
  }

  private closeSession(): void {
    this.session?.events.abort()
    this.session = null
    this.pendingPermissions.clear()
  }

  async send(payload: CompanionSendPayload, targetWindow: BrowserWindow | null): Promise<void> {
    if (this.turn) {
      this.emit({ kind: 'warning', message: 'Wait for the current response or stop it first.' })
      return
    }
    const turn: ActiveTurn = {
      messageId: randomUUID(),
      window: targetWindow,
      promptedAt: Date.now(),
      started: false,
    }
    this.turn = turn

    try {
      const runtime = await this.ensureRuntime()
      if (this.turn !== turn) return
      const directory = sessionDirectoryFor(payload)
      const session = await this.ensureSession(runtime, directory)
      // Stopped while OpenCode was starting: nothing is waiting for a response any more.
      if (this.turn !== turn) return
      session.translator.beginTurn()
      turn.promptedAt = Date.now()
      await runtime.client.prompt(session.id, composePrompt(payload, directory))
    } catch (error) {
      if (this.turn !== turn) return
      this.emit({
        kind: 'error',
        message: error instanceof Error ? error.message : 'Could not reach OpenCode.',
      })
      this.turn = null
    }
  }

  handleEvent(session: ActiveSession, event: OpencodeEvent): void {
    if (this.session !== session) return
    const result = session.translator.translate(event)
    if (result.started && this.turn) this.turn.started = true
    const live = this.turn?.started === true

    for (const update of result.updates) {
      if (!live) continue
      if (update.kind === 'change' && update.permissionId && update.status === 'pending') {
        this.pendingPermissions.add(update.permissionId)
      }
      this.emit(update)
    }

    if (result.autoReject && this.runtime) {
      const { permissionId, reason } = result.autoReject
      void this.runtime.client
        .replyPermission(session.id, permissionId, 'reject', reason)
        .catch(() => undefined)
    }

    if (result.finish && live) this.finishTurn(result.finish)
  }

  private finishTurn(finish: TurnFinish): void {
    if (!this.turn) return
    const { messageId } = this.turn
    if (finish.outcome === 'failed') {
      this.emit({ kind: 'error', message: finish.error ?? 'OpenCode failed.' })
    } else if (finish.outcome === 'interrupted') {
      this.emit({ kind: 'cancelled', messageId })
    } else {
      this.emit({ kind: 'done', messageId })
    }
    this.turn = null
    this.pendingPermissions.clear()
  }

  async replyPermission(
    permissionId: string,
    decision: CompanionPermissionDecision,
  ): Promise<void> {
    if (!this.session || !this.runtime || !this.pendingPermissions.has(permissionId)) {
      throw new Error('This change is no longer waiting for review.')
    }
    this.pendingPermissions.delete(permissionId)
    await this.runtime.client.replyPermission(
      this.session.id,
      permissionId,
      decision === 'approve' ? 'once' : 'reject',
      decision === 'reject' ? 'The user rejected this change.' : undefined,
    )
  }

  async cancel(): Promise<void> {
    const turn = this.turn
    const session = this.session
    const runtime = this.runtime
    this.turn = null
    if (session && runtime) {
      const pending = [...this.pendingPermissions]
      this.pendingPermissions.clear()
      await Promise.allSettled([
        runtime.client.interrupt(session.id),
        ...pending.map((id) =>
          runtime.client.replyPermission(
            session.id,
            id,
            'reject',
            'The user stopped the response.',
          ),
        ),
      ])
    }
    if (turn) {
      const win = turn.window ?? this.getMainWindow()
      if (win && !win.isDestroyed()) {
        win.webContents.send(IPC.COMPANION_UPDATE, {
          kind: 'cancelled',
          messageId: turn.messageId,
        } satisfies CompanionUpdate)
      }
    }
  }

  /** Starts a fresh conversation: the next message opens a new OpenCode session. */
  async reset(): Promise<void> {
    if (this.turn) await this.cancel()
    this.closeSession()
  }

  stopNow(): void {
    this.turn = null
    this.closeSession()
    const runtime = this.runtime
    this.runtime = null
    runtime?.server.stop()
  }

  async shutdown(): Promise<void> {
    if (this.turn) await this.cancel()
    this.closeSession()
    const runtime = this.runtime
    this.runtime = null
    runtime?.server.stop()
  }
}

let companionService: CompanionService | null = null

export function getCompanionService(getMainWindow: () => BrowserWindow | null): CompanionService {
  companionService ??= new CompanionService(getMainWindow)
  return companionService
}

/**
 * Stops the OpenCode server right away, for app quit: there is no time to wait for a running
 * response to be interrupted, and the server takes its sessions down with it.
 */
export function stopCompanionServiceNow(): void {
  companionService?.stopNow()
}
