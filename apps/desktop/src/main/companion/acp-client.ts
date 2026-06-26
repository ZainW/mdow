import { spawn } from 'node:child_process'
import type { Readable, Writable } from 'node:stream'

const ACP_CLIENT_INFO = { name: 'Mdow', version: '1.4.0' }

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

type AcpProcessListener =
  | ((error: Error) => void)
  | ((code: number | null, signal: string | null) => void)

type AcpProcess = {
  stdin: Writable
  stdout: Readable
  stderr: Readable
  kill: () => boolean
  on: (event: 'error' | 'exit' | 'close', listener: AcpProcessListener) => AcpProcess
  off: (event: 'error' | 'exit' | 'close', listener: AcpProcessListener) => AcpProcess
}

export type AcpProcessFactory = (
  command: string,
  args: string[],
  options: { cwd: string; env: NodeJS.ProcessEnv },
) => AcpProcess

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

const defaultProcessFactory: AcpProcessFactory = (command, args, options) =>
  spawn(command, args, { cwd: options.cwd, env: options.env, stdio: 'pipe' })

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
  let listeners:
    | {
        process: AcpProcess
        stdoutData: (chunk: Buffer | string) => void
        processError: (error: Error) => void
        processExit: (code: number | null, signal: string | null) => void
        processClose: (code: number | null, signal: string | null) => void
      }
    | undefined
  const pending = new Map<
    JsonRpcId,
    { resolve: (value: unknown) => void; reject: (reason: Error) => void }
  >()

  function rejectPending(error: Error) {
    for (const waiter of pending.values()) {
      waiter.reject(error)
    }
    pending.clear()
  }

  function detachListeners() {
    if (!listeners) {
      return
    }
    const activeListeners = listeners
    activeListeners.process.stdout.off('data', activeListeners.stdoutData)
    activeListeners.process.off('error', activeListeners.processError)
    activeListeners.process.off('exit', activeListeners.processExit)
    activeListeners.process.off('close', activeListeners.processClose)
    listeners = undefined
  }

  function cleanup({ kill, rejectWith }: { kill: boolean; rejectWith?: Error }) {
    const processToCleanup = child
    detachListeners()
    child = undefined
    sessionId = undefined
    stdoutBuffer = ''
    if (rejectWith) {
      rejectPending(rejectWith)
    } else {
      pending.clear()
    }
    if (kill) {
      processToCleanup?.kill()
    }
  }

  function reportProcessFailure(process: AcpProcess, error: Error) {
    if (process !== child) {
      return
    }
    onUpdate({ type: 'error', message: error.message })
    cleanup({ kill: false, rejectWith: error })
  }

  function reportProtocolFailure(error: Error) {
    onUpdate({ type: 'error', message: error.message })
    if (pending.size > 0) {
      cleanup({ kill: true, rejectWith: error })
    }
  }

  function send(message: JsonRpcRequest | JsonRpcNotification | JsonRpcResponse) {
    const process = child
    if (!process) {
      return Promise.reject(new Error('ACP process is not running'))
    }
    return writeMessage(process, message)
  }

  function request(method: string, params?: unknown) {
    const id = nextId++
    return new Promise<unknown>((resolve, reject) => {
      pending.set(id, { resolve, reject })
      send({ jsonrpc: '2.0', id, method, params }).catch((error: unknown) => {
        pending.delete(id)
        reject(error instanceof Error ? error : new Error(String(error)))
      })
    })
  }

  function notify(method: string, params?: unknown) {
    void send({ jsonrpc: '2.0', method, params }).catch((error: unknown) => {
      onUpdate({ type: 'error', message: error instanceof Error ? error.message : String(error) })
    })
  }

  function handleResponse(message: JsonRpcResponse) {
    const waiter = pending.get(message.id)
    if (!waiter) {
      return
    }
    pending.delete(message.id)
    if (message.error) {
      if (!isJsonRpcError(message.error)) {
        const error = new Error('Invalid JSON-RPC error response')
        onUpdate({ type: 'error', message: error.message })
        waiter.reject(error)
        return
      }
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
      void send({ jsonrpc: '2.0', id: message.id, result: refusalOutcome(message.params) }).catch(
        (error: unknown) => {
          onUpdate({
            type: 'error',
            message: error instanceof Error ? error.message : String(error),
          })
        },
      )
      return
    }

    void send({
      jsonrpc: '2.0',
      id: message.id,
      error: { code: -32601, message: 'Method not found' },
    }).catch((error: unknown) => {
      onUpdate({ type: 'error', message: error instanceof Error ? error.message : String(error) })
    })
  }

  function refusalOutcome(params: unknown) {
    const optionId = rejectOptionId(params)
    if (optionId) {
      return { outcome: { outcome: 'selected', optionId } }
    }
    return { outcome: { outcome: 'cancelled' } }
  }

  function rejectOptionId(params: unknown): string | null {
    if (!isRecord(params) || !Array.isArray(params.options)) {
      return null
    }

    for (const option of params.options) {
      if (!isRecord(option) || !isRejectOptionKind(option.kind)) {
        continue
      }
      if (typeof option.optionId === 'string') return option.optionId
      if (typeof option.id === 'string') return option.id
    }
    return null
  }

  function isRejectOptionKind(kind: unknown): boolean {
    if (typeof kind === 'string') {
      return kind.toLowerCase().includes('reject')
    }
    if (isRecord(kind) && typeof kind.kind === 'string') {
      return kind.kind.toLowerCase().includes('reject')
    }
    return false
  }

  function handleMessage(message: unknown) {
    if (!isRecord(message) || message.jsonrpc !== '2.0') {
      reportProtocolFailure(new Error('Invalid JSON-RPC message'))
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

    reportProtocolFailure(new Error('Invalid JSON-RPC message'))
  }

  function handleStdoutChunk(process: AcpProcess, chunk: Buffer | string) {
    if (process !== child) {
      return
    }
    stdoutBuffer += chunk.toString()
    const lines = stdoutBuffer.split('\n')
    stdoutBuffer = lines.pop() ?? ''

    for (const line of lines) {
      if (!line.trim()) {
        continue
      }
      try {
        handleMessage(JSON.parse(line))
      } catch {
        reportProtocolFailure(new Error('Malformed JSON-RPC message'))
      }
    }
  }

  return {
    async start() {
      if (child) {
        throw new Error('ACP client is already starting or started')
      }

      const providerProcess = processFactory(command, args, { cwd, env: process.env })
      child = providerProcess
      const stdoutData = (chunk: Buffer | string) => handleStdoutChunk(providerProcess, chunk)
      const processError = (error: Error) => reportProcessFailure(providerProcess, error)
      const handleProcessExit = (code: number | null, signal: string | null) => {
        reportProcessFailure(providerProcess, processExitError(code, signal))
      }
      listeners = {
        process: providerProcess,
        stdoutData,
        processError,
        processExit: handleProcessExit,
        processClose: handleProcessExit,
      }
      providerProcess.stdout.on('data', stdoutData)
      providerProcess.on('error', processError)
      providerProcess.on('exit', handleProcessExit)
      providerProcess.on('close', handleProcessExit)

      try {
        const initializeResult = await request('initialize', {
          protocolVersion: 1,
          clientCapabilities: {},
          clientInfo: ACP_CLIENT_INFO,
        })
        if (!isRecord(initializeResult) || initializeResult.protocolVersion !== 1) {
          throw new Error('Unsupported ACP protocol version')
        }

        await send({ jsonrpc: '2.0', method: 'initialized', params: {} })

        const sessionResult = await request('session/new', { cwd, mcpServers: [] })
        if (!isRecord(sessionResult) || typeof sessionResult.sessionId !== 'string') {
          throw new Error('ACP provider did not create a session')
        }
        sessionId = sessionResult.sessionId
      } catch (error) {
        cleanup({ kill: providerProcess === child, rejectWith: undefined })
        throw error
      }
    },

    async sendPrompt(prompt: AcpContentBlock[]) {
      if (!sessionId) {
        throw new Error('ACP session has not started')
      }
      await request('session/prompt', { sessionId, prompt })
    },

    cancel() {
      if (!sessionId) {
        return
      }
      notify('session/cancel', { sessionId })
    },

    stop() {
      cleanup({ kill: true, rejectWith: new Error('ACP client stopped') })
    },
  }
}

function writeMessage(
  process: AcpProcess,
  message: JsonRpcRequest | JsonRpcNotification | JsonRpcResponse,
) {
  return new Promise<void>((resolve, reject) => {
    let settled = false
    function settle(error?: Error | null) {
      if (settled) {
        return
      }
      settled = true
      if (error) {
        reject(error)
        return
      }
      resolve()
    }

    try {
      process.stdin.write(`${JSON.stringify(message)}\n`, 'utf8', settle)
    } catch (error) {
      settle(error instanceof Error ? error : new Error(String(error)))
    }
  })
}

function processExitError(code: number | null, signal: string | null) {
  if (signal) {
    return new Error(`ACP provider exited with signal ${signal}`)
  }
  if (code !== null) {
    return new Error(`ACP provider exited with code ${code}`)
  }
  return new Error('ACP provider exited')
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

function isJsonRpcError(
  value: unknown,
): value is { code: number; message: string; data?: unknown } {
  return isRecord(value) && typeof value.code === 'number' && typeof value.message === 'string'
}

function isJsonRpcRequest(value: Record<string, unknown>): value is JsonRpcRequest {
  return isJsonRpcId(value.id) && typeof value.method === 'string'
}

function isJsonRpcNotification(value: Record<string, unknown>): value is JsonRpcNotification {
  return !('id' in value) && typeof value.method === 'string'
}
