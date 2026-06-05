import { EventEmitter } from 'node:events'
import { PassThrough, Writable } from 'node:stream'
import { describe, expect, it, vi } from 'vitest'
import { createAcpClient, type AcpProcessFactory } from './acp-client'

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
  const factory: AcpProcessFactory = () => child
  return { child, stdout, writes, factory }
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

describe('createAcpClient', () => {
  it('initializes, creates a session, and sends text prompts', async () => {
    const { stdout, writes, factory } = createHarness()
    const client = createAcpClient({
      command: 'agent',
      args: ['--stdio'],
      cwd: '/workspace',
      processFactory: factory,
      onUpdate: vi.fn(),
    })

    const started = client.start()
    await waitForWriteCount(writes, 1)
    let messages = readMessages(writes)
    expect(messages[0]).toMatchObject({
      jsonrpc: '2.0',
      id: 1,
      method: 'initialize',
      params: {
        protocolVersion: 1,
        clientCapabilities: {},
        clientInfo: { name: 'Mdow' },
      },
    })

    writeJson(stdout, { jsonrpc: '2.0', id: 1, result: { protocolVersion: 1 } })
    await waitForWriteCount(writes, 2)
    messages = readMessages(writes)
    expect(messages[1]).toMatchObject({
      jsonrpc: '2.0',
      id: 2,
      method: 'session/new',
      params: { cwd: '/workspace', mcpServers: [] },
    })

    writeJson(stdout, { jsonrpc: '2.0', id: 2, result: { sessionId: 'session-1' } })
    await started

    await client.sendPrompt('Summarize this')
    messages = readMessages(writes)
    expect(messages[2]).toMatchObject({
      jsonrpc: '2.0',
      id: 3,
      method: 'session/prompt',
      params: {
        sessionId: 'session-1',
        prompt: [{ type: 'text', text: 'Summarize this' }],
      },
    })
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
    await waitForWriteCount(writes, 2)
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
      id: 99,
      method: 'session/request_permission',
      params: { sessionId: 'session-1', title: 'Edit file' },
    })

    await vi.waitFor(() => {
      expect(onUpdate).toHaveBeenCalledWith({ type: 'assistant-delta', text: 'Hello' })
      expect(onUpdate).toHaveBeenCalledWith({ type: 'tool-refused', title: 'Read file' })
      expect(onUpdate).toHaveBeenCalledWith({ type: 'tool-refused', title: 'Edit file' })
    })
    await waitForWriteCount(writes, 3)
    expect(readMessages(writes)[2]).toEqual({
      jsonrpc: '2.0',
      id: 99,
      result: { outcome: 'rejected' },
    })
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
    await waitForWriteCount(writes, 2)
    writeJson(stdout, { jsonrpc: '2.0', id: 2, result: { sessionId: 'session-1' } })
    await started

    client.cancel()
    await waitForWriteCount(writes, 3)
    expect(readMessages(writes)[2]).toEqual({
      jsonrpc: '2.0',
      method: 'session/cancel',
      params: { sessionId: 'session-1' },
    })

    client.stop()
    expect(child.kill).toHaveBeenCalledTimes(1)
  })
})
