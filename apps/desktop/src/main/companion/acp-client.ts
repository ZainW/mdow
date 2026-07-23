import { spawn, type ChildProcessWithoutNullStreams } from 'child_process'
import type { CompanionUpdate } from '../../shared/types'

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

interface PendingRequest {
  resolve: (value: unknown) => void
  reject: (error: Error) => void
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
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

function textFromSessionUpdate(update: unknown): string | null {
  if (!isRecord(update)) return null
  const kind = update.sessionUpdate ?? update.type
  if (kind === 'agent_message_chunk' || kind === 'agent_thought_chunk') {
    if (isRecord(update.content)) {
      if (typeof update.content.text === 'string') return update.content.text
    }
    const nested = extractTextFromContent(update.content)
    if (nested) return nested
  }
  if (kind === 'message' && typeof update.text === 'string') return update.text
  if (typeof update.text === 'string') return update.text
  return null
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
    child.stderr.on('data', (chunk: string) => {
      if (chunk.trim()) {
        this.onUpdate({ kind: 'status', message: chunk.trim().slice(0, 240) })
      }
    })
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
        version: '1.5.1',
      },
      clientCapabilities: {
        fs: {
          readTextFile: true,
          writeTextFile: false,
        },
        terminal: false,
      },
    })

    await this.notify('notifications/initialized', {})
  }

  async createSession(cwd: string): Promise<string> {
    const result = await this.request('session/new', {
      cwd,
      mcpServers: [],
    })
    if (!isRecord(result) || typeof result.sessionId !== 'string') {
      throw new Error('ACP session/new missing sessionId')
    }
    this.sessionId = result.sessionId
    return result.sessionId
  }

  getSessionId(): string | null {
    return this.sessionId
  }

  async prompt(text: string): Promise<void> {
    if (!this.sessionId) throw new Error('No ACP session')
    await this.request('session/prompt', {
      sessionId: this.sessionId,
      prompt: [{ type: 'text', text }],
    })
  }

  async cancel(): Promise<void> {
    if (!this.sessionId) return
    await this.notify('session/cancel', { sessionId: this.sessionId })
  }

  async shutdown(): Promise<void> {
    this.closed = true
    this.failAll(new Error('ACP client shut down'))
    const child = this.process
    this.process = null
    if (!child) return
    child.stdin.end()
    child.kill()
  }

  private failAll(error: Error): void {
    for (const pending of this.pending.values()) pending.reject(error)
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
      if ('error' in message && message.error) {
        pending.reject(new Error(message.error.message))
      } else if ('result' in message) {
        pending.resolve(message.result)
      }
      return
    }

    if ('method' in message && typeof message.method === 'string') {
      void this.handleIncoming(
        message.method,
        message.params,
        'id' in message ? message.id : undefined,
      )
    }
  }

  private async handleIncoming(
    method: string,
    params: unknown,
    id: JsonRpcId | undefined,
  ): Promise<void> {
    if (method === 'session/update') {
      if (isRecord(params)) {
        const text = textFromSessionUpdate(params.update ?? params)
        if (text) this.onUpdate({ kind: 'delta', text })
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

  private request(method: string, params: unknown): Promise<unknown> {
    const id = this.nextId++
    const payload: JsonRpcRequest = { jsonrpc: '2.0', id, method, params }
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject })
      try {
        this.send(payload)
      } catch (err) {
        this.pending.delete(id)
        reject(err instanceof Error ? err : new Error(String(err)))
      }
    })
  }

  private async notify(method: string, params: unknown): Promise<void> {
    this.send({ jsonrpc: '2.0', method, params })
  }

  private send(message: object): void {
    if (!this.process?.stdin.writable) {
      throw new Error('ACP process is not writable')
    }
    this.process.stdin.write(`${JSON.stringify(message)}\n`)
  }
}
