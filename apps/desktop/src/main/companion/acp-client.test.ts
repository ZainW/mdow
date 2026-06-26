import { EventEmitter } from 'node:events'
import { PassThrough, Writable } from 'node:stream'
import { describe, expect, it, vi } from 'vitest'
import { createAcpClient, type AcpContentBlock, type AcpProcessFactory } from './acp-client'

function createHarness() {
  const stdout = new PassThrough()
  const stderr = new PassThrough()
  const writes: string[] = []
  const stdin = new Writable({
    write(chunk: Buffer | string, _encoding, callback) {
      writes.push(chunk.toString())
      callback()
    },
  })
  const child = Object.assign(new EventEmitter(), {
    stdin,
    stdout,
    stderr,
    kill: vi.fn<() => boolean>(() => true),
  })
  const factory = vi.fn<AcpProcessFactory>(() => child)
  return { child, stdout, writes, factory }
}

function createFailingWriteHarness() {
  const stdout = new PassThrough()
  const stderr = new PassThrough()
  const stdin = new Writable({
    write(_chunk, _encoding, callback) {
      callback(new Error('stdin closed'))
    },
  })
  stdin.on('error', () => {})
  const child = Object.assign(new EventEmitter(), {
    stdin,
    stdout,
    stderr,
    kill: vi.fn<() => boolean>(() => true),
  })
  const factory = vi.fn<AcpProcessFactory>(() => child)
  return { child, stdout, factory }
}

function writeJson(stdout: PassThrough, message: unknown) {
  stdout.write(`${JSON.stringify(message)}\n`)
}

function readMessages(writes: string[]): unknown[] {
  return writes.flatMap((write) =>
    write
      .trim()
      .split('\n')
      .filter(Boolean)
      .map((line) => JSON.parse(line) as unknown),
  )
}

async function waitForWriteCount(writes: string[], count: number) {
  await vi.waitFor(() => expect(readMessages(writes)).toHaveLength(count))
}

async function settleMicrotasks() {
  await new Promise<void>((resolve) => queueMicrotask(resolve))
}

describe('createAcpClient', () => {
  it('initializes, creates a session, and sends prompt blocks', async () => {
    const { stdout, writes, factory } = createHarness()
    const client = createAcpClient({
      command: 'agent',
      args: ['--stdio'],
      cwd: '/workspace',
      processFactory: factory,
      onUpdate: vi.fn(),
    })

    const started = client.start()
    expect(factory).toHaveBeenCalledWith('agent', ['--stdio'], {
      cwd: '/workspace',
      env: process.env,
    })
    await waitForWriteCount(writes, 1)
    let messages = readMessages(writes)
    expect(messages[0]).toMatchObject({
      jsonrpc: '2.0',
      id: 1,
      method: 'initialize',
      params: {
        protocolVersion: 1,
        clientCapabilities: {},
        clientInfo: { name: 'Mdow', version: '1.4.0' },
      },
    })

    writeJson(stdout, { jsonrpc: '2.0', id: 1, result: { protocolVersion: 1 } })
    await waitForWriteCount(writes, 3)
    messages = readMessages(writes)
    expect(messages[1]).toMatchObject({
      jsonrpc: '2.0',
      method: 'initialized',
      params: {},
    })
    expect(messages[2]).toMatchObject({
      jsonrpc: '2.0',
      id: 2,
      method: 'session/new',
      params: { cwd: '/workspace', mcpServers: [] },
    })

    writeJson(stdout, { jsonrpc: '2.0', id: 2, result: { sessionId: 'session-1' } })
    await started

    const prompt: AcpContentBlock[] = [
      { type: 'text', text: 'Summarize this' },
      {
        type: 'resource',
        resource: {
          uri: 'file:///workspace/readme.md',
          mimeType: 'text/markdown',
          text: '# Readme',
        },
      },
    ]

    let promptSettled = false
    const sentPrompt = client.sendPrompt(prompt).then(() => {
      promptSettled = true
    })
    await waitForWriteCount(writes, 4)
    await settleMicrotasks()
    expect(promptSettled).toBe(false)

    messages = readMessages(writes)
    expect(messages[3]).toMatchObject({
      jsonrpc: '2.0',
      id: 3,
      method: 'session/prompt',
      params: {
        sessionId: 'session-1',
        prompt,
      },
    })

    writeJson(stdout, { jsonrpc: '2.0', id: 3, result: {} })
    await sentPrompt
    expect(promptSettled).toBe(true)
    client.stop()
  })

  it('rejects sendPrompt when the provider returns a prompt error', async () => {
    const { stdout, writes, factory } = createHarness()
    const client = createAcpClient({
      command: 'agent',
      args: [],
      cwd: '/workspace',
      processFactory: factory,
      onUpdate: vi.fn(),
    })

    const started = client.start()
    await waitForWriteCount(writes, 1)
    writeJson(stdout, { jsonrpc: '2.0', id: 1, result: { protocolVersion: 1 } })
    await waitForWriteCount(writes, 3)
    writeJson(stdout, { jsonrpc: '2.0', id: 2, result: { sessionId: 'session-1' } })
    await started

    const sentPrompt = client.sendPrompt([{ type: 'text', text: 'Fail please' }])
    await waitForWriteCount(writes, 4)
    writeJson(stdout, {
      jsonrpc: '2.0',
      id: 3,
      error: { code: -32000, message: 'prompt failed' },
    })

    await expect(sentPrompt).rejects.toThrow('prompt failed')
    client.stop()
  })

  it('streams agent message chunks and refuses permission requests', async () => {
    const onUpdate = vi.fn()
    const { stdout, writes, factory } = createHarness()
    const client = createAcpClient({
      command: 'agent',
      args: [],
      cwd: '/workspace',
      processFactory: factory,
      onUpdate,
    })

    const started = client.start()
    await waitForWriteCount(writes, 1)
    writeJson(stdout, { jsonrpc: '2.0', id: 1, result: { protocolVersion: 1 } })
    await waitForWriteCount(writes, 3)
    writeJson(stdout, { jsonrpc: '2.0', id: 2, result: { sessionId: 'session-1' } })
    await started

    writeJson(stdout, {
      jsonrpc: '2.0',
      method: 'session/update',
      params: {
        sessionId: 'session-1',
        update: {
          sessionUpdate: 'agent_message_chunk',
          content: { type: 'text', text: 'Hello' },
        },
      },
    })
    writeJson(stdout, {
      jsonrpc: '2.0',
      method: 'session/update',
      params: {
        sessionId: 'session-1',
        update: {
          sessionUpdate: 'tool_call',
          toolCall: { title: 'Read file' },
        },
      },
    })
    writeJson(stdout, {
      jsonrpc: '2.0',
      method: 'session/update',
      params: {
        sessionId: 'session-1',
        update: {
          sessionUpdate: 'tool_call_delta',
          toolCall: { title: 'Streaming tool call' },
        },
      },
    })
    writeJson(stdout, {
      jsonrpc: '2.0',
      id: 99,
      method: 'session/request_permission',
      params: { sessionId: 'session-1', title: 'Edit file' },
    })

    await vi.waitFor(() => {
      expect(onUpdate).toHaveBeenCalledWith({ type: 'assistant-delta', text: 'Hello' })
      expect(onUpdate).toHaveBeenCalledWith({ type: 'tool-refused', title: 'Read file' })
      expect(onUpdate).toHaveBeenCalledWith({ type: 'tool-refused', title: 'Edit file' })
      expect(onUpdate).not.toHaveBeenCalledWith({
        type: 'tool-refused',
        title: 'Streaming tool call',
      })
    })
    await waitForWriteCount(writes, 4)
    expect(readMessages(writes)[3]).toEqual({
      jsonrpc: '2.0',
      id: 99,
      result: { outcome: { outcome: 'cancelled' } },
    })
    client.stop()
  })

  it('selects reject permission options when refusing tool requests', async () => {
    const { stdout, writes, factory } = createHarness()
    const client = createAcpClient({
      command: 'agent',
      args: [],
      cwd: '/workspace',
      processFactory: factory,
      onUpdate: vi.fn(),
    })

    const started = client.start()
    await waitForWriteCount(writes, 1)
    writeJson(stdout, { jsonrpc: '2.0', id: 1, result: { protocolVersion: 1 } })
    await waitForWriteCount(writes, 3)
    writeJson(stdout, { jsonrpc: '2.0', id: 2, result: { sessionId: 'session-1' } })
    await started

    writeJson(stdout, {
      jsonrpc: '2.0',
      id: 99,
      method: 'session/request_permission',
      params: {
        sessionId: 'session-1',
        title: 'Edit file',
        options: [
          { optionId: 'allow-once', kind: 'AllowOnce' },
          { optionId: 'reject-once', kind: { kind: 'RejectOnce' } },
        ],
      },
    })

    await waitForWriteCount(writes, 4)
    expect(readMessages(writes)[3]).toEqual({
      jsonrpc: '2.0',
      id: 99,
      result: { outcome: { outcome: 'selected', optionId: 'reject-once' } },
    })
    client.stop()
  })

  it('sends session cancel notifications and kills the process on stop', async () => {
    const { child, stdout, writes, factory } = createHarness()
    const client = createAcpClient({
      command: 'agent',
      args: [],
      cwd: '/workspace',
      processFactory: factory,
      onUpdate: vi.fn(),
    })

    const started = client.start()
    await waitForWriteCount(writes, 1)
    writeJson(stdout, { jsonrpc: '2.0', id: 1, result: { protocolVersion: 1 } })
    await waitForWriteCount(writes, 3)
    writeJson(stdout, { jsonrpc: '2.0', id: 2, result: { sessionId: 'session-1' } })
    await started

    client.cancel()
    await waitForWriteCount(writes, 4)
    expect(readMessages(writes)[3]).toEqual({
      jsonrpc: '2.0',
      method: 'session/cancel',
      params: { sessionId: 'session-1' },
    })

    client.stop()
    expect(child.kill).toHaveBeenCalledTimes(1)
  })

  it('rejects pending start and reports an error when the process errors', async () => {
    const onUpdate = vi.fn()
    const { child, writes, factory } = createHarness()
    const client = createAcpClient({
      command: 'agent',
      args: [],
      cwd: '/workspace',
      processFactory: factory,
      onUpdate,
    })

    const started = client.start()
    await waitForWriteCount(writes, 1)
    child.emit('error', new Error('spawn failed'))

    await expect(started).rejects.toThrow('spawn failed')
    expect(onUpdate).toHaveBeenCalledWith({ type: 'error', message: 'spawn failed' })
  })

  it('rejects pending start and clears session state when the process exits', async () => {
    const onUpdate = vi.fn()
    const { child, writes, factory } = createHarness()
    const client = createAcpClient({
      command: 'agent',
      args: [],
      cwd: '/workspace',
      processFactory: factory,
      onUpdate,
    })

    const started = client.start()
    await waitForWriteCount(writes, 1)
    child.emit('exit', 1, null)

    await expect(started).rejects.toThrow('ACP provider exited')
    expect(onUpdate).toHaveBeenCalledWith({
      type: 'error',
      message: 'ACP provider exited with code 1',
    })
    await expect(client.sendPrompt([{ type: 'text', text: 'After exit' }])).rejects.toThrow(
      'ACP session has not started',
    )
  })

  it('kills and detaches listeners when handshake validation fails', async () => {
    const onUpdate = vi.fn()
    const { child, stdout, writes, factory } = createHarness()
    const client = createAcpClient({
      command: 'agent',
      args: [],
      cwd: '/workspace',
      processFactory: factory,
      onUpdate,
    })

    const started = client.start()
    await waitForWriteCount(writes, 1)
    writeJson(stdout, { jsonrpc: '2.0', id: 1, result: { protocolVersion: 2 } })

    await expect(started).rejects.toThrow('Unsupported ACP protocol version')
    expect(child.kill).toHaveBeenCalledTimes(1)

    writeJson(stdout, {
      jsonrpc: '2.0',
      method: 'session/update',
      params: {
        update: {
          sessionUpdate: 'agent_message_chunk',
          content: { type: 'text', text: 'stale' },
        },
      },
    })
    expect(onUpdate).not.toHaveBeenCalledWith({ type: 'assistant-delta', text: 'stale' })
  })

  it('ignores stale stdout after stop', async () => {
    const onUpdate = vi.fn()
    const { stdout, writes, factory } = createHarness()
    const client = createAcpClient({
      command: 'agent',
      args: [],
      cwd: '/workspace',
      processFactory: factory,
      onUpdate,
    })

    const started = client.start()
    await waitForWriteCount(writes, 1)
    writeJson(stdout, { jsonrpc: '2.0', id: 1, result: { protocolVersion: 1 } })
    await waitForWriteCount(writes, 3)
    writeJson(stdout, { jsonrpc: '2.0', id: 2, result: { sessionId: 'session-1' } })
    await started

    client.stop()
    writeJson(stdout, {
      jsonrpc: '2.0',
      method: 'session/update',
      params: {
        update: {
          sessionUpdate: 'agent_message_chunk',
          content: { type: 'text', text: 'after stop' },
        },
      },
    })

    expect(onUpdate).not.toHaveBeenCalledWith({ type: 'assistant-delta', text: 'after stop' })
  })

  it('rejects start when stdin write fails', async () => {
    const onUpdate = vi.fn()
    const { child, factory } = createFailingWriteHarness()
    const client = createAcpClient({
      command: 'agent',
      args: [],
      cwd: '/workspace',
      processFactory: factory,
      onUpdate,
    })

    await expect(client.start()).rejects.toThrow('stdin closed')
    expect(child.kill).toHaveBeenCalledTimes(1)
  })

  it('rejects malformed JSON-RPC error responses safely', async () => {
    const onUpdate = vi.fn()
    const { stdout, writes, factory } = createHarness()
    const client = createAcpClient({
      command: 'agent',
      args: [],
      cwd: '/workspace',
      processFactory: factory,
      onUpdate,
    })

    const started = client.start()
    await waitForWriteCount(writes, 1)
    writeJson(stdout, { jsonrpc: '2.0', id: 1, error: { code: -1 } })

    await expect(started).rejects.toThrow('Invalid JSON-RPC error response')
    expect(onUpdate).toHaveBeenCalledWith({
      type: 'error',
      message: 'Invalid JSON-RPC error response',
    })
  })

  it('rejects pending start and cleans up when provider writes malformed JSON', async () => {
    const onUpdate = vi.fn()
    const { child, stdout, writes, factory } = createHarness()
    const client = createAcpClient({
      command: 'agent',
      args: [],
      cwd: '/workspace',
      processFactory: factory,
      onUpdate,
    })

    const started = client.start()
    await waitForWriteCount(writes, 1)
    stdout.write('{not json}\n')

    await expect(started).rejects.toThrow('Malformed JSON-RPC message')
    expect(onUpdate).toHaveBeenCalledWith({
      type: 'error',
      message: 'Malformed JSON-RPC message',
    })
    expect(child.kill).toHaveBeenCalledTimes(1)
  })

  it('rejects pending start and cleans up when provider writes invalid JSON-RPC', async () => {
    const onUpdate = vi.fn()
    const { child, stdout, writes, factory } = createHarness()
    const client = createAcpClient({
      command: 'agent',
      args: [],
      cwd: '/workspace',
      processFactory: factory,
      onUpdate,
    })

    const started = client.start()
    await waitForWriteCount(writes, 1)
    writeJson(stdout, { jsonrpc: '2.0' })

    await expect(started).rejects.toThrow('Invalid JSON-RPC message')
    expect(onUpdate).toHaveBeenCalledWith({
      type: 'error',
      message: 'Invalid JSON-RPC message',
    })
    expect(child.kill).toHaveBeenCalledTimes(1)
  })

  it('rejects double start without spawning another process', async () => {
    const harness = createHarness()
    const factory = vi.fn<AcpProcessFactory>(() => harness.child)
    const client = createAcpClient({
      command: 'agent',
      args: [],
      cwd: '/workspace',
      processFactory: factory,
      onUpdate: vi.fn(),
    })

    const started = client.start()
    await waitForWriteCount(harness.writes, 1)

    await expect(client.start()).rejects.toThrow('ACP client is already starting or started')
    expect(factory).toHaveBeenCalledTimes(1)

    harness.child.emit('error', new Error('stop pending start'))
    await expect(started).rejects.toThrow('stop pending start')
  })
})
