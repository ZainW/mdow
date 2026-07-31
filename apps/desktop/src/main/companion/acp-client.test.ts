import { EventEmitter } from 'events'
import { Readable, Writable } from 'stream'
import { describe, expect, it, vi } from 'vitest'
import type { CompanionUpdate } from '../../shared/types'
import { AcpClient } from './acp-client'

class FakeChild extends EventEmitter {
  stdin = new Writable({
    write: (chunk, _enc, cb) => {
      this.written.push(String(chunk))
      queueMicrotask(() => this.respondTo(String(chunk)))
      cb()
    },
  })
  stdout = new Readable({ read() {} })
  stderr = new Readable({ read() {} })
  written: string[] = []
  private requestCount = 0
  ignoredMethods = new Set<string>()

  respondTo(chunk: string): void {
    for (const line of chunk.split('\n').filter(Boolean)) {
      const msg = JSON.parse(line) as {
        id?: number
        method?: string
        params?: Record<string, unknown>
      }
      if (msg.method && this.ignoredMethods.has(msg.method)) continue
      if (msg.method === 'initialize' && msg.id !== undefined) {
        this.stdout.push(
          `${JSON.stringify({
            jsonrpc: '2.0',
            id: msg.id,
            result: { protocolVersion: 1, agentCapabilities: {} },
          })}\n`,
        )
      } else if (msg.method === 'session/new' && msg.id !== undefined) {
        this.stdout.push(
          `${JSON.stringify({
            jsonrpc: '2.0',
            id: msg.id,
            result: { sessionId: 'sess_test' },
          })}\n`,
        )
      } else if (msg.method === 'session/prompt' && msg.id !== undefined) {
        this.requestCount += 1
        this.stdout.push(
          `${JSON.stringify({
            jsonrpc: '2.0',
            method: 'session/update',
            params: {
              sessionId: 'sess_test',
              update: {
                sessionUpdate: 'agent_thought_chunk',
                content: { type: 'text', text: 'Thinking…' },
              },
            },
          })}\n`,
        )
        this.stdout.push(
          `${JSON.stringify({
            jsonrpc: '2.0',
            method: 'session/update',
            params: {
              sessionId: 'sess_test',
              update: {
                sessionUpdate: 'tool_call',
                toolCallId: 'tool_1',
                title: 'read',
                status: 'completed',
                rawInput: { path: 'a.md' },
                rawOutput: { result: 'ok' },
              },
            },
          })}\n`,
        )
        this.stdout.push(
          `${JSON.stringify({
            jsonrpc: '2.0',
            method: 'session/update',
            params: {
              sessionId: 'sess_test',
              update: {
                sessionUpdate: 'agent_message_chunk',
                content: { type: 'text', text: 'Hello from agent' },
              },
            },
          })}\n`,
        )
        this.stdout.push(
          `${JSON.stringify({
            jsonrpc: '2.0',
            id: msg.id,
            result: { stopReason: 'end_turn' },
          })}\n`,
        )
      } else if (msg.method === 'fs/write_text_file' && msg.id !== undefined) {
        // ignored; client handles incoming only
      }
    }
  }

  kill(): void {
    this.emit('exit', 0)
  }
}

describe('Companion ACP client', () => {
  it('initializes, creates a session, and streams prompt deltas', async () => {
    const updates: CompanionUpdate[] = []
    const fake = new FakeChild()
    const client = new AcpClient({
      command: 'fake-agent',
      args: [],
      onUpdate: (u) => updates.push(u),
      spawnImpl: (() => fake) as unknown as typeof import('child_process').spawn,
    })

    await client.start()
    const writtenMessages = fake.written
      .flatMap((chunk) => chunk.split('\n'))
      .filter(Boolean)
      .map(
        (line) =>
          JSON.parse(line) as {
            method?: string
            params?: {
              clientInfo?: { version?: string }
              clientCapabilities?: { fs?: { readTextFile?: boolean } }
            }
          },
      )
    const initialize = writtenMessages.find((message) => message.method === 'initialize')
    expect(initialize?.params?.clientInfo?.version).toBe('1.5.2')
    expect(initialize?.params?.clientCapabilities?.fs?.readTextFile).toBe(false)
    expect(writtenMessages.map((message) => message.method)).not.toContain(
      'notifications/initialized',
    )

    await client.createSession('/tmp/docs')
    await client.prompt('What is this?')

    expect(client.getSessionId()).toBe('sess_test')
    expect(updates.some((u) => u.kind === 'thinking' && u.text === 'Thinking…')).toBe(true)
    expect(updates.some((u) => u.kind === 'thinking-done')).toBe(true)
    expect(
      updates.some(
        (u) =>
          u.kind === 'tool' &&
          u.toolCallId === 'tool_1' &&
          u.name === 'read' &&
          u.state === 'completed' &&
          u.output === '{\n  "result": "ok"\n}',
      ),
    ).toBe(true)
    expect(updates.some((u) => u.kind === 'delta' && u.text === 'Hello from agent')).toBe(true)
    await client.shutdown()
  })

  it('does not surface subprocess diagnostics as conversation status', async () => {
    const updates: CompanionUpdate[] = []
    const fake = new FakeChild()
    const client = new AcpClient({
      command: 'fake-agent',
      args: [],
      onUpdate: (update) => updates.push(update),
      spawnImpl: (() => fake) as unknown as typeof import('child_process').spawn,
    })

    await client.start()
    fake.stderr.push('agent diagnostic output\n')
    await new Promise((resolve) => setImmediate(resolve))

    expect(updates).not.toContainEqual({
      kind: 'status',
      message: 'agent diagnostic output',
    })
    await client.shutdown()
  })

  it('times out unanswered ACP requests and clears them', async () => {
    const fake = new FakeChild()
    const client = new AcpClient({
      command: 'fake-agent',
      args: [],
      onUpdate: vi.fn(),
      spawnImpl: (() => fake) as unknown as typeof import('child_process').spawn,
    })
    await client.start()
    fake.ignoredMethods.add('session/new')
    vi.useFakeTimers()

    let rejection: Error | undefined
    const pending = client.createSession('/tmp/docs').catch((error: unknown) => {
      rejection = error instanceof Error ? error : new Error(String(error))
    })
    try {
      await vi.advanceTimersByTimeAsync(30_001)
      expect(rejection?.message).toMatch(/session\/new timed out/i)
    } finally {
      await client.shutdown()
      await pending
      vi.useRealTimers()
    }
  })

  it('does not apply the setup timeout to a long-running prompt', async () => {
    const fake = new FakeChild()
    const client = new AcpClient({
      command: 'fake-agent',
      args: [],
      onUpdate: vi.fn(),
      spawnImpl: (() => fake) as unknown as typeof import('child_process').spawn,
    })
    await client.start()
    await client.createSession('/tmp/docs')
    fake.ignoredMethods.add('session/prompt')
    vi.useFakeTimers()

    let settled = false
    const pending = client.prompt('Take your time').finally(() => {
      settled = true
    })
    try {
      await vi.advanceTimersByTimeAsync(30_001)
      expect(settled).toBe(false)
    } finally {
      await client.shutdown()
      await expect(pending).rejects.toThrow(/shut down/i)
      vi.useRealTimers()
    }
  })

  it('refuses write and terminal requests from the agent', async () => {
    const updates: { kind: string; message?: string }[] = []
    const fake = new FakeChild()
    const client = new AcpClient({
      command: 'fake-agent',
      args: [],
      onUpdate: (u) => updates.push(u),
      spawnImpl: (() => fake) as unknown as typeof import('child_process').spawn,
    })
    await client.start()

    fake.stdout.push(
      `${JSON.stringify({
        jsonrpc: '2.0',
        id: 99,
        method: 'fs/write_text_file',
        params: { path: '/tmp/x.md', content: 'nope' },
      })}\n`,
    )

    await vi.waitFor(() => {
      expect(updates.some((u) => u.kind === 'warning' && u.message?.includes('fs/write'))).toBe(
        true,
      )
    })

    const reply = fake.written.find((line) => line.includes('"id":99'))
    expect(reply).toBeTruthy()
    expect(reply).toContain('Refused')
    await client.shutdown()
  })
})
