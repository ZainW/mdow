import { spawn } from 'node:child_process'
import type { Readable, Writable } from 'node:stream'

type JsonRpcId = number | string

type JsonRpcRequest = {
  jsonrpc: '2.0'
  id: JsonRpcId
  method: string
  params?: unknown
}

type JsonRpcNotification = {
  jsonrpc: '2.0'
  method: string
  params?: unknown
}

type JsonRpcResponse = {
  jsonrpc: '2.0'
  id: JsonRpcId
  result?: unknown
  error?: { code: number; message: string; data?: unknown }
}

type AcpProcess = {
  stdin: Writable
  stdout: Readable
  stderr: Readable
  kill: () => boolean
}

export type AcpProcessFactory = (command: string, args: string[], cwd: string) => AcpProcess

export type AcpContentBlock =
  | { type: 'text'; text: string }
  | { type: 'resource'; resource: { uri: string; mimeType?: string; text: string } }

export type AcpClientUpdate =
  | { type: 'assistant-delta'; text: string }
  | { type: 'tool-refused'; title: string }
  | { type: 'error'; message: string }

export type AcpClient = {
  start: () => Promise<void>
  sendPrompt: (prompt: AcpContentBlock[]) => Promise<void>
  cancel: () => void
  stop: () => void
}

type CreateAcpClientOptions = {
  command: string
  args: string[]
  cwd: string
  processFactory?: AcpProcessFactory
  onUpdate: (update: AcpClientUpdate) => void
}

const defaultProcessFactory: AcpProcessFactory = (command, args, cwd) =>
  spawn(command, args, { cwd, stdio: 'pipe' })

export function createAcpClient({
  command,
  args,
  cwd,
  processFactory = defaultProcessFactory,
  onUpdate,
}: CreateAcpClientOptions): AcpClient {
  let child: AcpProcess | undefined
  let nextId = 1
  let sessionId: string | undefined
  let stdoutBuffer = ''
  const pending = new Map<
    JsonRpcId,
    { resolve: (value: unknown) => void; reject: (reason: Error) => void }
  >()

  function send(message: JsonRpcRequest | JsonRpcNotification | JsonRpcResponse) {
    child?.stdin.write(`${JSON.stringify(message)}\n`, 'utf8')
  }

  function request(method: string, params?: unknown) {
    const id = nextId++
    send({ jsonrpc: '2.0', id, method, params })
    return new Promise<unknown>((resolve, reject) => {
      pending.set(id, { resolve, reject })
    })
  }

  function notify(method: string, params?: unknown) {
    send({ jsonrpc: '2.0', method, params })
  }

  function handleResponse(message: JsonRpcResponse) {
    const waiter = pending.get(message.id)
    if (!waiter) {
      return
    }
    pending.delete(message.id)
    if (message.error) {
      waiter.reject(new Error(message.error.message))
      return
    }
    waiter.resolve(message.result)
  }

  function updatePayload(params: unknown) {
    if (!isRecord(params)) {
      return undefined
    }
    const nested = params.update
    return isRecord(nested) ? nested : params
  }

  function extractTitle(value: unknown) {
    if (!isRecord(value)) {
      return 'Tool call refused'
    }
    if (typeof value.title === 'string') {
      return value.title
    }
    if (isRecord(value.toolCall) && typeof value.toolCall.title === 'string') {
      return value.toolCall.title
    }
    return 'Tool call refused'
  }

  function handleNotification(message: JsonRpcNotification) {
    if (message.method !== 'session/update') {
      return
    }

    const update = updatePayload(message.params)
    if (!update) {
      return
    }

    if (update.sessionUpdate === 'agent_message_chunk') {
      const content = update.content
      if (isRecord(content) && content.type === 'text' && typeof content.text === 'string') {
        onUpdate({ type: 'assistant-delta', text: content.text })
      }
      return
    }

    if (update.sessionUpdate === 'tool_call') {
      onUpdate({ type: 'tool-refused', title: extractTitle(update) })
    }
  }

  function handleRequest(message: JsonRpcRequest) {
    if (message.method === 'session/request_permission') {
      onUpdate({ type: 'tool-refused', title: extractTitle(message.params) })
      send({ jsonrpc: '2.0', id: message.id, result: { outcome: 'rejected' } })
      return
    }

    send({
      jsonrpc: '2.0',
      id: message.id,
      error: { code: -32601, message: 'Method not found' },
    })
  }

  function handleMessage(message: unknown) {
    if (!isRecord(message) || message.jsonrpc !== '2.0') {
      onUpdate({ type: 'error', message: 'Invalid JSON-RPC message' })
      return
    }

    if (isJsonRpcResponse(message)) {
      handleResponse(message)
      return
    }

    if (isJsonRpcRequest(message)) {
      handleRequest(message)
      return
    }

    if (isJsonRpcNotification(message)) {
      handleNotification(message)
      return
    }

    onUpdate({ type: 'error', message: 'Invalid JSON-RPC message' })
  }

  function handleStdoutChunk(chunk: Buffer | string) {
    stdoutBuffer += chunk.toString()
    const lines = stdoutBuffer.split('\n')
    stdoutBuffer = lines.pop() ?? ''

    for (const line of lines) {
      if (!line.trim()) {
        continue
      }
      try {
        handleMessage(JSON.parse(line))
      } catch (error) {
        onUpdate({
          type: 'error',
          message: error instanceof Error ? error.message : 'Malformed JSON-RPC message',
        })
      }
    }
  }

  return {
    async start() {
      child = processFactory(command, args, cwd)
      child.stdout.on('data', handleStdoutChunk)

      const initializeResult = await request('initialize', {
        protocolVersion: 1,
        clientCapabilities: {},
        clientInfo: { name: 'Mdow' },
      })
      if (!isRecord(initializeResult) || initializeResult.protocolVersion !== 1) {
        throw new Error('Unsupported ACP protocol version')
      }

      const sessionResult = await request('session/new', { cwd, mcpServers: [] })
      if (!isRecord(sessionResult) || typeof sessionResult.sessionId !== 'string') {
        throw new Error('ACP provider did not create a session')
      }
      sessionId = sessionResult.sessionId
    },

    sendPrompt(prompt: AcpContentBlock[]) {
      if (!sessionId) {
        return Promise.reject(new Error('ACP session has not started'))
      }
      request('session/prompt', { sessionId, prompt }).catch((error: unknown) => {
        onUpdate({ type: 'error', message: error instanceof Error ? error.message : String(error) })
      })
      return Promise.resolve()
    },

    cancel() {
      if (!sessionId) {
        return
      }
      notify('session/cancel', { sessionId })
    },

    stop() {
      child?.kill()
      child = undefined
      sessionId = undefined
      stdoutBuffer = ''
      for (const waiter of pending.values()) {
        waiter.reject(new Error('ACP client stopped'))
      }
      pending.clear()
    },
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function isJsonRpcId(value: unknown): value is JsonRpcId {
  return typeof value === 'number' || typeof value === 'string'
}

function isJsonRpcResponse(value: Record<string, unknown>): value is JsonRpcResponse {
  return isJsonRpcId(value.id) && ('result' in value || 'error' in value)
}

function isJsonRpcRequest(value: Record<string, unknown>): value is JsonRpcRequest {
  return isJsonRpcId(value.id) && typeof value.method === 'string'
}

function isJsonRpcNotification(value: Record<string, unknown>): value is JsonRpcNotification {
  return !('id' in value) && typeof value.method === 'string'
}
