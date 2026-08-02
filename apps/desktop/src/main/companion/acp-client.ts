import { spawn, type ChildProcessWithoutNullStreams } from 'child_process'
import packageJson from '../../../package.json' with { type: 'json' }
import type {
  CompanionModelOption,
  CompanionModelProvider,
  CompanionModelState,
  CompanionUpdate,
} from '../../shared/types'

type JsonRpcId = number | string

interface JsonRpcRequest {
  jsonrpc: '2.0'
  id: JsonRpcId
  method: string
  params?: unknown
}

interface JsonRpcNotification {
  jsonrpc: '2.0'
  method: string
  params?: unknown
}

interface JsonRpcSuccess {
  jsonrpc: '2.0'
  id: JsonRpcId
  result: unknown
}

interface JsonRpcFailure {
  jsonrpc: '2.0'
  id: JsonRpcId
  error: { code: number; message: string; data?: unknown }
}

type IncomingMessage = JsonRpcRequest | JsonRpcNotification | JsonRpcSuccess | JsonRpcFailure

export type AcpUpdateHandler = (update: CompanionUpdate) => void

export interface AcpClientOptions {
  command: string
  args: string[]
  cwd?: string
  onUpdate: AcpUpdateHandler
  spawnImpl?: typeof spawn
}

export interface AcpMcpServer {
  name: string
  command: string
  args: string[]
  env: Array<{ name: string; value: string }>
}

export interface AcpSessionState {
  sessionId: string
  configOptions: unknown[]
}

interface PendingRequest {
  resolve: (value: unknown) => void
  reject: (error: Error) => void
  timeout: ReturnType<typeof setTimeout> | null
}

interface AcpConfigOptionValue {
  value: string
  name: string
  description?: string
}

interface AcpConfigOption {
  id: string
  name: string
  category?: string
  type: string
  currentValue: string | boolean
  options?: AcpConfigOptionValue[]
}

const REQUEST_TIMEOUT_MS = 30_000

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function parseConfigOptions(value: unknown): AcpConfigOption[] {
  if (!Array.isArray(value)) return []
  const options: AcpConfigOption[] = []
  for (const candidate of value) {
    if (
      !isRecord(candidate) ||
      typeof candidate.id !== 'string' ||
      typeof candidate.name !== 'string' ||
      typeof candidate.type !== 'string' ||
      (typeof candidate.currentValue !== 'string' && typeof candidate.currentValue !== 'boolean')
    ) {
      continue
    }
    const values = Array.isArray(candidate.options)
      ? candidate.options.flatMap((option) => {
          if (!isRecord(option) || typeof option.value !== 'string') return []
          return [
            {
              value: option.value,
              name: typeof option.name === 'string' ? option.name : option.value,
              ...(typeof option.description === 'string'
                ? { description: option.description }
                : {}),
            },
          ]
        })
      : undefined
    options.push({
      id: candidate.id,
      name: candidate.name,
      ...(typeof candidate.category === 'string' ? { category: candidate.category } : {}),
      type: candidate.type,
      currentValue: candidate.currentValue,
      ...(values ? { options: values } : {}),
    })
  }
  return options
}

function modelProvider(value: string): CompanionModelProvider | null {
  if (value.startsWith('openai/')) return 'openai'
  if (value.startsWith('opencode-go/')) return 'opencode-go'
  if (value.startsWith('opencode/')) return 'opencode'
  return null
}

function modelOptionFrom(configOptions: AcpConfigOption[]): AcpConfigOption | null {
  return (
    configOptions.find((option) => option.category === 'model') ??
    configOptions.find((option) => option.id === 'model') ??
    null
  )
}

function toModelState(configOptions: AcpConfigOption[], hasSession: boolean): CompanionModelState {
  if (!hasSession) {
    return {
      options: [],
      currentValue: null,
      stale: true,
      unavailableReason: 'Start Companion to load models',
    }
  }
  const modelConfig = modelOptionFrom(configOptions)
  if (!modelConfig || modelConfig.type !== 'select') {
    return {
      options: [],
      currentValue: null,
      stale: false,
      unavailableReason: 'The current provider does not expose model selection',
    }
  }
  const options: CompanionModelOption[] = (modelConfig.options ?? []).flatMap((option) => {
    const provider = modelProvider(option.value)
    if (!provider) return []
    return [
      {
        value: option.value,
        name: option.name,
        ...(option.description ? { description: option.description } : {}),
        provider,
      },
    ]
  })
  const currentValue =
    typeof modelConfig.currentValue === 'string' &&
    options.some((option) => option.value === modelConfig.currentValue)
      ? modelConfig.currentValue
      : null
  return {
    options,
    currentValue,
    stale: false,
    ...(options.length === 0
      ? { unavailableReason: 'No supported OpenCode models are live in this session' }
      : {}),
  }
}

function extractTextFromContent(content: unknown): string {
  if (typeof content === 'string') return content
  if (!Array.isArray(content)) return ''
  const parts: string[] = []
  for (const block of content) {
    if (!isRecord(block)) continue
    if (block.type === 'text' && typeof block.text === 'string') parts.push(block.text)
  }
  return parts.join('')
}

function textFromSessionUpdate(
  update: unknown,
): { channel: 'message' | 'thinking'; text: string } | null {
  if (!isRecord(update)) return null
  const kind = update.sessionUpdate ?? update.type
  if (kind === 'agent_thought_chunk') {
    let text = ''
    if (isRecord(update.content) && typeof update.content.text === 'string') {
      text = update.content.text
    } else {
      text = extractTextFromContent(update.content)
    }
    return text ? { channel: 'thinking', text } : null
  }
  if (kind === 'agent_message_chunk' || kind === 'message') {
    let text = ''
    if (isRecord(update.content) && typeof update.content.text === 'string') {
      text = update.content.text
    } else if (typeof update.text === 'string') {
      text = update.text
    } else {
      text = extractTextFromContent(update.content)
    }
    return text ? { channel: 'message', text } : null
  }
  if (typeof update.text === 'string') return { channel: 'message', text: update.text }
  return null
}

function toolFromSessionUpdate(update: unknown): CompanionUpdate | null {
  if (!isRecord(update)) return null
  const kind = update.sessionUpdate ?? update.type
  if (kind !== 'tool_call' && kind !== 'tool_call_update') return null

  const toolCallId =
    (typeof update.toolCallId === 'string' && update.toolCallId) ||
    (typeof update.id === 'string' && update.id) ||
    crypto.randomUUID()
  const name =
    (typeof update.title === 'string' && update.title) ||
    (typeof update.kind === 'string' && update.kind) ||
    (typeof update.name === 'string' && update.name) ||
    'tool'

  let state: 'pending' | 'running' | 'completed' | 'error' | 'cancelled' = 'running'
  const rawStatus = typeof update.status === 'string' ? update.status : ''
  if (rawStatus === 'pending' || rawStatus === 'in_progress') state = 'running'
  if (rawStatus === 'completed' || rawStatus === 'success') state = 'completed'
  if (rawStatus === 'failed' || rawStatus === 'error') state = 'error'
  if (rawStatus === 'cancelled') state = 'cancelled'
  if (kind === 'tool_call' && !rawStatus) state = 'pending'

  const input =
    typeof update.rawInput === 'string'
      ? update.rawInput
      : update.rawInput !== undefined
        ? JSON.stringify(update.rawInput, null, 2)
        : undefined
  const output =
    typeof update.rawOutput === 'string'
      ? update.rawOutput
      : update.rawOutput !== undefined
        ? JSON.stringify(update.rawOutput, null, 2)
        : update.content !== undefined
          ? JSON.stringify(update.content, null, 2)
          : undefined

  return {
    kind: 'tool',
    toolCallId,
    name,
    state,
    input,
    output,
  }
}

export class AcpClient {
  private process: ChildProcessWithoutNullStreams | null = null
  private buffer = ''
  private nextId = 1
  private readonly pending = new Map<JsonRpcId, PendingRequest>()
  private sessionId: string | null = null
  private readonly onUpdate: AcpUpdateHandler
  private readonly command: string
  private readonly args: string[]
  private readonly cwd: string | undefined
  private readonly spawnImpl: typeof spawn
  private closed = false
  private lastTextChannel: 'message' | 'thinking' | null = null
  private configOptions: AcpConfigOption[] = []

  constructor(options: AcpClientOptions) {
    this.command = options.command
    this.args = options.args
    this.cwd = options.cwd
    this.onUpdate = options.onUpdate
    this.spawnImpl = options.spawnImpl ?? spawn
  }

  async start(): Promise<void> {
    if (this.process) return
    const child = this.spawnImpl(this.command, this.args, {
      cwd: this.cwd,
      stdio: ['pipe', 'pipe', 'pipe'],
      env: process.env,
    })
    this.process = child

    child.stdout.setEncoding('utf8')
    child.stdout.on('data', (chunk: string) => this.onStdout(chunk))
    child.stderr.setEncoding('utf8')
    child.stderr.on('data', () => undefined)
    child.on('error', (err) => {
      this.failAll(err)
      this.onUpdate({ kind: 'error', message: err.message })
    })
    child.on('exit', (code) => {
      this.process = null
      if (!this.closed) {
        this.failAll(new Error(`ACP process exited (${code ?? 'null'})`))
      }
    })

    await this.request('initialize', {
      protocolVersion: 1,
      clientInfo: {
        name: 'mdow',
        title: 'Mdow',
        version: packageJson.version,
      },
      clientCapabilities: {
        fs: {
          readTextFile: false,
          writeTextFile: false,
        },
        terminal: false,
      },
    })
  }

  async createSession(cwd: string, mcpServers: AcpMcpServer[] = []): Promise<AcpSessionState> {
    const result = await this.request('session/new', {
      cwd,
      mcpServers,
    })
    if (!isRecord(result) || typeof result.sessionId !== 'string') {
      throw new Error('ACP session/new missing sessionId')
    }
    this.sessionId = result.sessionId
    this.configOptions = parseConfigOptions(result.configOptions)
    return {
      sessionId: result.sessionId,
      configOptions: this.configOptions,
    }
  }

  getSessionId(): string | null {
    return this.sessionId
  }

  getModelState(): CompanionModelState {
    return toModelState(this.configOptions, Boolean(this.sessionId))
  }

  async setModel(value: string): Promise<CompanionModelState> {
    if (!this.sessionId) throw new Error('No ACP session')
    const modelConfig = modelOptionFrom(this.configOptions)
    const state = this.getModelState()
    if (!modelConfig || !state.options.some((option) => option.value === value)) {
      throw new Error(`Model is not available in this session: ${value}`)
    }
    const result = await this.request('session/set_config_option', {
      sessionId: this.sessionId,
      configId: modelConfig.id,
      value,
    })
    if (!isRecord(result) || !Array.isArray(result.configOptions)) {
      throw new Error('ACP session/set_config_option missing configOptions')
    }
    this.configOptions = parseConfigOptions(result.configOptions)
    return this.getModelState()
  }

  async prompt(text: string): Promise<void> {
    if (!this.sessionId) throw new Error('No ACP session')
    this.lastTextChannel = null
    await this.request(
      'session/prompt',
      {
        sessionId: this.sessionId,
        prompt: [{ type: 'text', text }],
      },
      null,
    )
  }

  async cancel(): Promise<void> {
    if (!this.sessionId) return
    await this.notify('session/cancel', { sessionId: this.sessionId })
  }

  shutdown(): Promise<void> {
    this.closed = true
    this.configOptions = []
    this.sessionId = null
    this.failAll(new Error('ACP client shut down'))
    const child = this.process
    this.process = null
    if (!child) return Promise.resolve()
    child.stdin.end()
    child.kill()
    return Promise.resolve()
  }

  private failAll(error: Error): void {
    for (const pending of this.pending.values()) {
      if (pending.timeout) clearTimeout(pending.timeout)
      pending.reject(error)
    }
    this.pending.clear()
  }

  private onStdout(chunk: string): void {
    this.buffer += chunk
    while (true) {
      const newline = this.buffer.indexOf('\n')
      if (newline === -1) break
      const line = this.buffer.slice(0, newline).trim()
      this.buffer = this.buffer.slice(newline + 1)
      if (!line) continue
      this.handleLine(line)
    }
  }

  private handleLine(line: string): void {
    let message: IncomingMessage
    try {
      message = JSON.parse(line) as IncomingMessage
    } catch {
      this.onUpdate({ kind: 'warning', message: 'Ignored malformed ACP message' })
      return
    }

    if (
      'id' in message &&
      message.id !== undefined &&
      ('result' in message || 'error' in message)
    ) {
      const pending = this.pending.get(message.id)
      if (!pending) return
      this.pending.delete(message.id)
      if (pending.timeout) clearTimeout(pending.timeout)
      if ('error' in message && message.error) {
        pending.reject(new Error(message.error.message))
      } else if ('result' in message) {
        pending.resolve(message.result)
      }
      return
    }

    if ('method' in message && typeof message.method === 'string') {
      this.handleIncoming(message.method, message.params, 'id' in message ? message.id : undefined)
    }
  }

  private handleIncoming(method: string, params: unknown, id: JsonRpcId | undefined): void {
    if (method === 'session/update') {
      if (isRecord(params)) {
        const update = params.update ?? params
        if (
          isRecord(update) &&
          (update.sessionUpdate === 'config_option_update' ||
            update.type === 'config_option_update')
        ) {
          this.configOptions = parseConfigOptions(update.configOptions)
          return
        }
        const tool = toolFromSessionUpdate(update)
        if (tool) {
          if (this.lastTextChannel === 'thinking') {
            this.onUpdate({ kind: 'thinking-done' })
            this.lastTextChannel = null
          }
          this.onUpdate(tool)
          return
        }
        const textUpdate = textFromSessionUpdate(update)
        if (textUpdate?.channel === 'thinking') {
          this.onUpdate({ kind: 'thinking', text: textUpdate.text })
          this.lastTextChannel = 'thinking'
        } else if (textUpdate?.channel === 'message') {
          if (this.lastTextChannel === 'thinking') {
            this.onUpdate({ kind: 'thinking-done' })
          }
          this.onUpdate({ kind: 'delta', text: textUpdate.text })
          this.lastTextChannel = 'message'
        }
      }
      return
    }

    if (method === 'fs/write_text_file' || method.startsWith('terminal/')) {
      if (id !== undefined) {
        this.send({
          jsonrpc: '2.0',
          id,
          error: { code: -32000, message: 'Refused in read-only companion mode' },
        })
      }
      this.onUpdate({
        kind: 'warning',
        message: `Refused agent request: ${method}`,
      })
      return
    }

    if (method === 'fs/read_text_file') {
      if (id !== undefined) {
        this.send({
          jsonrpc: '2.0',
          id,
          error: {
            code: -32000,
            message: 'Use provided docs context; direct fs reads are not enabled yet',
          },
        })
      }
      return
    }

    if (method === 'session/request_permission') {
      if (id !== undefined) {
        this.send({
          jsonrpc: '2.0',
          id,
          result: { outcome: { outcome: 'cancelled' } },
        })
      }
      return
    }

    if (id !== undefined) {
      this.send({
        jsonrpc: '2.0',
        id,
        error: { code: -32601, message: `Method not supported: ${method}` },
      })
    }
  }

  private request(
    method: string,
    params: unknown,
    timeoutMs: number | null = REQUEST_TIMEOUT_MS,
  ): Promise<unknown> {
    const id = this.nextId++
    const payload: JsonRpcRequest = { jsonrpc: '2.0', id, method, params }
    return new Promise((resolve, reject) => {
      const timeout =
        timeoutMs === null
          ? null
          : setTimeout(() => {
              this.pending.delete(id)
              reject(new Error(`ACP ${method} timed out after ${timeoutMs}ms`))
            }, timeoutMs)
      this.pending.set(id, { resolve, reject, timeout })
      try {
        this.send(payload)
      } catch (err) {
        if (timeout) clearTimeout(timeout)
        this.pending.delete(id)
        reject(err instanceof Error ? err : new Error(String(err)))
      }
    })
  }

  private notify(method: string, params: unknown): Promise<void> {
    this.send({ jsonrpc: '2.0', method, params })
    return Promise.resolve()
  }

  private send(message: object): void {
    if (!this.process?.stdin.writable) {
      throw new Error('ACP process is not writable')
    }
    this.process.stdin.write(`${JSON.stringify(message)}\n`)
  }
}
